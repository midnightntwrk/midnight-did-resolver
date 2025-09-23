macro_rules! wrap_field_arith {
    ($wrapper_name:ident) => {
        impl std::ops::Add for $wrapper_name {
            type Output = $wrapper_name;

            fn add(self, other: $wrapper_name) -> $wrapper_name {
                $wrapper_name(self.0 + other.0)
            }
        }

        impl std::ops::Sub for $wrapper_name {
            type Output = $wrapper_name;

            fn sub(self, other: $wrapper_name) -> $wrapper_name {
                $wrapper_name(self.0 - other.0)
            }
        }

        impl std::ops::Mul for $wrapper_name {
            type Output = $wrapper_name;

            fn mul(self, other: $wrapper_name) -> $wrapper_name {
                $wrapper_name(self.0 * other.0)
            }
        }

        impl std::ops::Neg for $wrapper_name {
            type Output = $wrapper_name;

            fn neg(self) -> $wrapper_name {
                $wrapper_name(-self.0)
            }
        }

        impl std::ops::Div for $wrapper_name {
            type Output = $wrapper_name;

            fn div(self, rhs: $wrapper_name) -> $wrapper_name {
                self.mul($wrapper_name(rhs.0.invert().unwrap()))
            }
        }
    };
}

macro_rules! wrap_group_arith {
    ($wrapper_name:ident, $scalar_wrapper:ident) => {
        impl std::ops::Add for $wrapper_name {
            type Output = $wrapper_name;

            fn add(self, other: $wrapper_name) -> $wrapper_name {
                $wrapper_name((self.0 + other.0).into())
            }
        }

        impl std::ops::Sub for $wrapper_name {
            type Output = $wrapper_name;

            fn sub(self, other: $wrapper_name) -> $wrapper_name {
                $wrapper_name((self.0 - other.0).into())
            }
        }

        impl std::ops::Mul<$scalar_wrapper> for $wrapper_name {
            type Output = $wrapper_name;

            fn mul(self, other: $scalar_wrapper) -> $wrapper_name {
                $wrapper_name(self.0.mul(other.0).into())
            }
        }

        impl std::ops::Neg for $wrapper_name {
            type Output = $wrapper_name;

            fn neg(self) -> $wrapper_name {
                $wrapper_name(-self.0)
            }
        }
    };
}

macro_rules! fr_display {
    ($wrapper_name:ident) => {
        impl std::fmt::Display for $wrapper_name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                let mut repr = self.as_le_bytes();
                while let Some(0) = repr.last() {
                    repr.pop();
                }
                if repr.is_empty() {
                    write!(formatter, "-")?;
                } else {
                    for byte in repr {
                        write!(formatter, "{byte:02x}")?;
                    }
                }
                Ok(())
            }
        }

        impl std::fmt::Debug for $wrapper_name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                std::fmt::Display::fmt(&self, formatter)
            }
        }
    };
}

macro_rules! wrap_display {
    ($wrapper_name:ident) => {
        impl std::fmt::Display for $wrapper_name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                std::fmt::Debug::fmt(&self.0, formatter)
            }
        }
        impl std::fmt::Debug for $wrapper_name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                std::fmt::Debug::fmt(&self.0, formatter)
            }
        }
    };
}

pub(crate) use {fr_display, wrap_display, wrap_field_arith, wrap_group_arith};
