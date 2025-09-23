#![deny(warnings)]

#[cfg(test)]
mod tests {
    use borsh::{BorshDeserialize, BorshSerialize};
    use midnight_serialize::{self as serialize, *};
    use std::collections::HashMap;
    use std::sync::Arc;

    #[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
    struct SerializableStruct {
        foo: u64,
        bar: u64,
    }

    const_serializable!(SerializableStruct, 128);

    #[derive(BorshSerialize, BorshDeserialize, PartialEq, Clone, Debug)]
    struct VersionedStruct {
        foo: u64,
        bar: u64,
    }

    impl Versioned for VersionedStruct {
        const VERSION: Option<Version> = Some(Version { major: 1, minor: 0 });
    }

    impl Serializable for VersionedStruct {
        fn unversioned_serialized_size(_value: &Self) -> usize {
            128
        }

        fn unversioned_serialize<W: std::io::Write>(
            value: &Self,
            writer: &mut W,
        ) -> Result<(), std::io::Error> {
            value.serialize(writer)
        }
    }

    impl Deserializable for VersionedStruct {
        fn versioned_deserialize<R: std::io::Read>(
            reader: &mut R,
            version: Option<&Version>,
            _recursion_depth: u32,
        ) -> Result<Self, std::io::Error> {
            match version {
                Some(Version { major: 1, minor: 0 }) => Ok(Self {
                    foo: u64::deserialize_reader(reader)?,
                    bar: u64::deserialize_reader(reader)?,
                }),
                _ => Err(Self::deserialization_error(
                    version,
                    "Unsupported version.".to_string(),
                )),
            }
        }
    }

    #[derive(PartialEq, Debug, Versioned, Serializable, Deserializable)]
    struct DerivedSerializable {
        serializable: SerializableStruct,
        versioned: VersionedStruct,
    }

    #[derive(PartialEq, Debug, Versioned, Serializable, Deserializable)]
    enum DerivedEnum {
        Unit,
        Tuple(u8, u8),
        Struct { foo: u8, bar: u8 },
    }

    #[test]
    fn serialize_serializable() {
        let mut writer = Vec::new();
        let value = SerializableStruct { foo: 5, bar: 10 };
        serialize(&value, &mut writer, NetworkId::Undeployed).unwrap();
        assert_eq!(
            value,
            deserialize(&mut writer.as_slice(), NetworkId::Undeployed).unwrap()
        );
    }

    #[test]
    fn serialize_versioned() {
        let mut writer = Vec::new();
        let value = VersionedStruct { foo: 2, bar: 5 };
        serialize(&value, &mut writer, NetworkId::Undeployed).unwrap();
        std::dbg!(&writer);

        assert_eq!(writer[1], 1); // Major Version
        assert_eq!(writer[2], 0); // Minor Version
        assert_eq!(
            value,
            deserialize(&mut writer.as_slice(), NetworkId::Undeployed).unwrap()
        );
    }

    #[test]
    fn serialize_vec_serializable() {
        let mut writer = Vec::new();
        let value: Vec<SerializableStruct> = vec![SerializableStruct { foo: 4, bar: 10 }; 3];
        serialize(&value, &mut writer, NetworkId::Undeployed).unwrap();
        assert_eq!(writer[0], 3); // Size
        let result: Vec<SerializableStruct> =
            deserialize(&mut writer.as_slice(), NetworkId::Undeployed).unwrap();
        assert_eq!(value, result);
    }

    #[test]
    fn serialize_vec_versioned() {
        let mut writer = Vec::new();
        let value: Vec<VersionedStruct> = vec![VersionedStruct { foo: 4, bar: 10 }; 3];
        serialize(&value, &mut writer, NetworkId::DevNet).unwrap();
        assert_eq!(writer[0], 1); // NetworkId
        assert_eq!(writer[1], 3); // Size
        let result: Vec<VersionedStruct> =
            deserialize(&mut writer.as_slice(), NetworkId::DevNet).unwrap();
        assert_eq!(value, result);
    }

    #[test]
    fn serialize_hashmap_serializable() {
        let mut writer = Vec::new();
        let mut value: HashMap<String, SerializableStruct> = HashMap::new();
        value.insert(String::from("test"), SerializableStruct { foo: 4, bar: 10 });
        value.insert(
            String::from("test1"),
            SerializableStruct { foo: 5, bar: 11 },
        );
        value.insert(
            String::from("test2"),
            SerializableStruct { foo: 6, bar: 12 },
        );
        serialize(&value, &mut writer, NetworkId::Undeployed).unwrap();
        assert_eq!(writer[0], 3); // Size
        let result: HashMap<String, SerializableStruct> =
            deserialize(&mut writer.as_slice(), NetworkId::Undeployed).unwrap();
        assert_eq!(value, result);
    }

    #[test]
    fn serialize_hashmap_versioned() {
        let mut writer = Vec::new();
        let mut value: HashMap<String, VersionedStruct> = HashMap::new();
        value.insert(String::from("test"), VersionedStruct { foo: 4, bar: 10 });
        value.insert(String::from("test1"), VersionedStruct { foo: 5, bar: 11 });
        value.insert(String::from("test2"), VersionedStruct { foo: 6, bar: 12 });
        serialize(&value, &mut writer, NetworkId::DevNet).unwrap();
        assert_eq!(writer[0], 1); // Network ID
        assert_eq!(writer[1], 3); // Size
        let result: HashMap<String, VersionedStruct> =
            deserialize(&mut writer.as_slice(), NetworkId::DevNet).unwrap();
        assert_eq!(value, result);
    }

    #[test]
    fn serialize_tuple() {
        let mut writer = Vec::new();
        let value: (SerializableStruct, VersionedStruct) = (
            SerializableStruct { foo: 5, bar: 11 },
            VersionedStruct { foo: 4, bar: 10 },
        );
        serialize(&value, &mut writer, NetworkId::Undeployed).unwrap();
        let result = deserialize(&mut writer.as_slice(), NetworkId::Undeployed).unwrap();
        assert_eq!(value, result);
    }

    #[test]
    fn serialize_ref_tuple() {
        let mut writer = Vec::new();
        let a = SerializableStruct { foo: 5, bar: 11 };
        let b = VersionedStruct { foo: 4, bar: 10 };
        let value = (&a, &b);
        serialize(&value, &mut writer, NetworkId::Undeployed).unwrap();
        let result: (SerializableStruct, VersionedStruct) =
            deserialize(&mut writer.as_slice(), NetworkId::Undeployed).unwrap();
        assert_eq!(*value.0, result.0);
        assert_eq!(*value.1, result.1);
    }

    #[test]
    fn error_on_too_few_bytes() {
        let mut writer = Vec::new();
        let value = SerializableStruct { foo: 5, bar: 11 };
        serialize(&value, &mut writer, NetworkId::Undeployed).unwrap();
        writer.pop();
        assert!(
            deserialize::<SerializableStruct, &[u8]>(writer.as_slice(), NetworkId::Undeployed)
                .is_err()
        );
    }

    #[test]
    fn error_on_left_over_bytes() {
        let mut writer = Vec::new();
        let value = SerializableStruct { foo: 5, bar: 11 };
        writer.push(0);
        serialize(&value, &mut writer, NetworkId::Undeployed).unwrap();
        assert!(
            deserialize::<SerializableStruct, &[u8]>(writer.as_slice(), NetworkId::Undeployed)
                .is_err()
        );
    }

    #[test]
    fn derive_serializable() {
        let mut writer = Vec::new();
        let value = DerivedSerializable {
            serializable: SerializableStruct { foo: 4, bar: 5 },
            versioned: VersionedStruct { foo: 6, bar: 7 },
        };
        serialize(&value, &mut writer, NetworkId::Undeployed).unwrap();
        assert_eq!(
            value,
            deserialize(&mut writer.as_slice(), NetworkId::Undeployed).unwrap()
        );
    }

    #[test]
    fn option_serializable() {
        let mut writer = Vec::new();

        let some_val: Option<VersionedStruct> = Some(VersionedStruct { foo: 2, bar: 3 });
        let none_val: Option<VersionedStruct> = None;

        serialize(&some_val, &mut writer, NetworkId::Undeployed).unwrap();
        assert_eq!(
            some_val,
            deserialize(&mut writer.as_slice(), NetworkId::Undeployed).unwrap()
        );

        writer = Vec::new();
        serialize(&none_val, &mut writer, NetworkId::Undeployed).unwrap();
        assert_eq!(
            none_val,
            deserialize(&mut writer.as_slice(), NetworkId::Undeployed).unwrap()
        );
    }

    #[test]
    fn serialized_size() {
        let ss = SerializableStruct { foo: 1, bar: 2 };
        let vs = VersionedStruct { foo: 3, bar: 4 };
        let ds = DerivedSerializable {
            serializable: ss.clone(),
            versioned: vs.clone(),
        };
        assert_eq!(128, SerializableStruct::serialized_size(&ss));
        assert_eq!(130, VersionedStruct::serialized_size(&vs));
        assert_eq!(258, DerivedSerializable::serialized_size(&ds));
    }

    #[test]
    fn enum_derive() {
        let unit = DerivedEnum::Unit;
        let tuple = DerivedEnum::Tuple(2, 3);
        let strct = DerivedEnum::Struct { foo: 1, bar: 4 };

        let mut writer = Vec::new();
        serialize(&unit, &mut writer, NetworkId::Undeployed).unwrap();
        assert_eq!(
            unit,
            deserialize(&mut writer.as_slice(), NetworkId::Undeployed).unwrap()
        );

        writer = Vec::new();
        serialize(&tuple, &mut writer, NetworkId::Undeployed).unwrap();
        assert_eq!(
            tuple,
            deserialize(&mut writer.as_slice(), NetworkId::Undeployed).unwrap()
        );

        writer = Vec::new();
        serialize(&strct, &mut writer, NetworkId::Undeployed).unwrap();
        assert_eq!(
            strct,
            deserialize(&mut writer.as_slice(), NetworkId::Undeployed).unwrap()
        );
    }

    #[test]
    fn recursive_limit() {
        enum Recursive {
            None,
            Some(Arc<Recursive>),
        }

        impl Versioned for Recursive {
            const VERSION: Option<Version> = None;
            const LIMIT_RECURSION: bool = true;
        }

        impl Serializable for Recursive {
            fn unversioned_serialize<W: std::io::Write>(
                value: &Self,
                writer: &mut W,
            ) -> Result<(), std::io::Error> {
                match value {
                    Recursive::None => <u8 as Serializable>::serialize(&0, writer),
                    Recursive::Some(child) => {
                        <u8 as Serializable>::serialize(&1, writer)?;
                        Recursive::serialize(child, writer)
                    }
                }
            }

            fn unversioned_serialized_size(value: &Self) -> usize {
                match value {
                    Recursive::None => 1,
                    Recursive::Some(child) => 1 + Recursive::serialized_size(child),
                }
            }
        }

        impl Deserializable for Recursive {
            fn versioned_deserialize<R: std::io::Read>(
                reader: &mut R,
                _version: Option<&Version>,
                recursion_depth: u32,
            ) -> Result<Self, std::io::Error> {
                let det = <u8 as Deserializable>::deserialize(reader, recursion_depth)?;
                match det {
                    0 => Ok(Recursive::None),
                    1 => Ok(Recursive::Some(Arc::new(Recursive::deserialize(
                        reader,
                        recursion_depth,
                    )?))),
                    _ => unreachable!(),
                }
            }
        }

        let mut value = Recursive::None;
        for _ in 0..(RECURSION_LIMIT + 1) {
            value = Recursive::Some(Arc::new(value));
        }

        let mut bytes: Vec<u8> = Vec::new();
        serialize(&value, &mut bytes, NetworkId::Undeployed).unwrap();
        let res: Result<Recursive, std::io::Error> =
            deserialize(&mut bytes.as_slice(), NetworkId::Undeployed);
        assert!(res.is_err())
    }
}
