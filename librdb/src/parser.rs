use std::{ffi::CString, marker::PhantomData, path::Path, ptr};

use librdb_sys::{
    self, RdbHandlersDataCallbacks, RdbStatus_RDB_STATUS_ERROR, RdbStatus_RDB_STATUS_OK,
};

use crate::{
    handlers::RdbHandlers,
    trampoline::{self, HandlerState},
    types::{RdbError, Result},
};

/// RAII wrapper around `RdbParser*`.
///
/// `H` implements [`RdbHandlers`] and receives callbacks during parsing.
/// The parser owns the handler and returns it via [`into_handler`](Self::into_handler)
/// after parsing completes.
pub struct Parser<H: RdbHandlers> {
    raw: *mut librdb_sys::RdbParser,
    state: *mut HandlerState<H>,
    _handlers: *mut librdb_sys::RdbHandlers,
    parsed: bool,
    // The underlying C parser is not thread-safe; Parser must stay on the creating thread.
    _not_send: PhantomData<*mut ()>,
}

#[allow(unsafe_code)]
impl<H: RdbHandlers> Parser<H> {
    /// Create a new parser that will dispatch callbacks to `handler`.
    ///
    /// # Errors
    /// Returns an error if the underlying C parser cannot be created.
    pub fn new(handler: H) -> Result<Self> {
        let state = Box::new(HandlerState {
            handler,
            last_error: None,
        });
        let state_ptr = Box::into_raw(state);

        // SAFETY: null memAlloc tells librdb to use default malloc/free.
        let raw = unsafe { librdb_sys::RDB_createParserRdb(ptr::null_mut()) };
        if raw.is_null() {
            // SAFETY: state_ptr was just created by Box::into_raw above.
            let _ = unsafe { Box::from_raw(state_ptr) };
            return Err(RdbError::Parser {
                code: 0,
                message: "RDB_createParserRdb returned null".into(),
            });
        }

        let mut callbacks: RdbHandlersDataCallbacks = trampoline::build_callbacks::<H>();

        // SAFETY: `raw` is a valid parser. `callbacks` is copied by librdb internally.
        // `state_ptr` outlives the parser. We pass `None` for `freeUserData` because
        // we reclaim `state_ptr` ourselves in `Drop`.
        let handlers = unsafe {
            librdb_sys::RDB_createHandlersData(
                raw,
                std::ptr::from_mut(&mut callbacks),
                state_ptr.cast(),
                None,
            )
        };

        if handlers.is_null() {
            // SAFETY: raw is valid; state_ptr was created above and not yet freed.
            unsafe { librdb_sys::RDB_deleteParser(raw) };
            let _ = unsafe { Box::from_raw(state_ptr) };
            return Err(RdbError::Parser {
                code: 0,
                message: "RDB_createHandlersData returned null".into(),
            });
        }

        Ok(Self {
            raw,
            state: state_ptr,
            _handlers: handlers,
            parsed: false,
            _not_send: PhantomData,
        })
    }

    /// Attach a file reader and parse the entire RDB file.
    ///
    /// Can only be called once per parser instance. A second call returns an error
    /// because librdb does not support re-parsing after completion.
    ///
    /// # Errors
    /// Returns an error if the file cannot be opened, the RDB data is malformed,
    /// or a handler callback returns `Err`.
    pub fn parse_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        if self.parsed {
            return Err(RdbError::Parser {
                code: 0,
                message: "parser already used; create a new Parser instance".into(),
            });
        }

        let c_path = CString::new(path.as_ref().to_str().ok_or_else(|| RdbError::Parser {
            code: 0,
            message: "path contains invalid UTF-8".into(),
        })?)
        .map_err(|_| RdbError::Parser {
            code: 0,
            message: "path contains null byte".into(),
        })?;

        // SAFETY: self.raw is a valid parser in CONFIGURING state, c_path is valid.
        let reader = unsafe { librdb_sys::RDBX_createReaderFile(self.raw, c_path.as_ptr()) };
        if reader.is_null() {
            return Err(self.extract_c_error("RDBX_createReaderFile returned null"));
        }

        self.parsed = true;
        self.run_parse()
    }

    fn run_parse(&mut self) -> Result<()> {
        // SAFETY: parser and reader are valid, callbacks are wired.
        let status = unsafe { librdb_sys::RDB_parse(self.raw) };

        if status == RdbStatus_RDB_STATUS_OK {
            return Ok(());
        }

        // SAFETY: self.state is valid and exclusively accessed from this thread.
        let state = unsafe { &mut *self.state };
        if let Some(handler_err) = state.last_error.take() {
            return Err(handler_err);
        }

        if status == RdbStatus_RDB_STATUS_ERROR {
            return Err(self.extract_c_error("parse failed"));
        }

        Err(RdbError::Parser {
            code: status,
            message: format!("unexpected parse status: {status}"),
        })
    }

    fn extract_c_error(&self, fallback: &str) -> RdbError {
        // SAFETY: self.raw is valid. librdb returns a null-terminated C string.
        let code = unsafe { librdb_sys::RDB_getErrorCode(self.raw) };
        let msg_ptr = unsafe { librdb_sys::RDB_getErrorMessage(self.raw) };
        let message = if msg_ptr.is_null() {
            fallback.to_owned()
        } else {
            // SAFETY: msg_ptr is a valid null-terminated string owned by the parser.
            unsafe { std::ffi::CStr::from_ptr(msg_ptr) }
                .to_string_lossy()
                .into_owned()
        };
        RdbError::Parser { code, message }
    }

    /// Borrow the handler, e.g. to inspect accumulated state after a parse failure.
    #[must_use]
    pub fn handler(&self) -> &H {
        // SAFETY: self.state is valid and not null while Parser is alive.
        unsafe { &(*self.state).handler }
    }

    /// Mutably borrow the handler.
    pub fn handler_mut(&mut self) -> &mut H {
        // SAFETY: self.state is valid, not null, and exclusively accessed.
        unsafe { &mut (*self.state).handler }
    }

    /// Consume the parser and return the handler.
    #[must_use]
    pub fn into_handler(mut self) -> H {
        // Null self.state so the subsequent Drop doesn't double-free the Box
        // we are about to reclaim here.
        let state_ptr = self.state;
        self.state = ptr::null_mut();
        // SAFETY: state_ptr was created by Box::into_raw in new() and has not been freed.
        let state = unsafe { Box::from_raw(state_ptr) };
        state.handler
    }
}

#[allow(unsafe_code)]
impl<H: RdbHandlers> Drop for Parser<H> {
    fn drop(&mut self) {
        // SAFETY: RDB_deleteParser frees the parser and all attached handlers/readers.
        unsafe { librdb_sys::RDB_deleteParser(self.raw) };

        if !self.state.is_null() {
            // SAFETY: state was created by Box::into_raw and not yet reclaimed.
            let _ = unsafe { Box::from_raw(self.state) };
        }
    }
}
