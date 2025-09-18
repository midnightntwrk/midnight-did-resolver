use midnight_ledger_v4::base_crypto::fab::Value;

struct CompactError(String);

trait CompactType: Sized {
    fn from_value(value: Value) -> Result<Self, CompactError>;
}

struct CompactTypeBoolean(bool);
struct CompactTypeBytes<const N: usize>([u8; N]);

impl CompactType for CompactTypeBoolean {
    fn from_value(mut value: Value) -> Result<Self, CompactError> {
        let maybe_val = value.0.pop().map(|i| i.0);
        let Some(val) = maybe_val else {
            Err(CompactError("expected Boolean".to_string()))?
        };
        if val.len() > 1 || (val.len() == 1 && val[0] != 1) {
            Err(CompactError("expected Boolean".to_string()))?
        }
        Ok(Self(val.len() == 1))
    }
}

impl<const N: usize> CompactType for CompactTypeBytes<N> {
    fn from_value(mut value: Value) -> Result<Self, CompactError> {
        let maybe_val = value.0.pop().map(|i| i.0);
        let Some(val) = maybe_val else {
            Err(CompactError(format!("expected Bytes[{N}]")))?
        };
        let Ok(array) = val.try_into() else {
            Err(CompactError(format!("expected Bytes[{N}]")))?
        };
        Ok(Self(array))
    }
}
