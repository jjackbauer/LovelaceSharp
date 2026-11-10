//! The native Windows shell.
//!
//! Two modules: ffi.rs declares the Win32 entry points and wraps them in safe functions, and
//! shell.rs builds the window, the panes, the menu, the accelerators and the message loop on top
//! of those wrappers. Nothing outside this directory contains unsafe code.

pub mod ffi;
pub mod shell;
