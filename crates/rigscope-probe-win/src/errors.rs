use rigscope_model::Field;
use rigscope_win::WinError;

/// The `Field` a failed read becomes.
pub fn field_err<T>(e: &WinError) -> Field<T> {
    match e {
        WinError::AccessDenied => Field::NeedsElevation,
        WinError::Timeout => Field::Unavailable("WMI did not answer".into()),
        WinError::Failed { api, code } => {
            Field::Unavailable(format!("{api} failed (0x{code:08X})"))
        }
        WinError::NotAvailable => Field::Unknown,
    }
}

/// `Ok(v)` -> `f(v)`, `Err(e)` -> [`field_err`].
pub fn field_of<T, U>(r: &Result<T, WinError>, f: impl FnOnce(&T) -> Field<U>) -> Field<U> {
    match r {
        Ok(v) => f(v),
        Err(e) => field_err(e),
    }
}
