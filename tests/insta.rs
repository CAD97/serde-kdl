use {
    serde::Serialize, serde_bytes::Bytes, serde_kdl::ser::to_string_ugly,
    std::collections::BTreeMap,
};

#[test]
fn ugly_primitives() -> serde_kdl::Result {
    insta::assert_display_snapshot!("ugly bool", to_string_ugly(&true)?);
    insta::assert_display_snapshot!("ugly u32", to_string_ugly(&0u32)?);
    insta::assert_display_snapshot!("ugly i32", to_string_ugly(&0i32)?);
    insta::assert_display_snapshot!("ugly f32", to_string_ugly(&0f32)?);
    insta::assert_display_snapshot!("ugly string", to_string_ugly("Hello")?);
    insta::assert_display_snapshot!("ugly bytes", to_string_ugly(Bytes::new(b"KDL"))?);
    insta::assert_display_snapshot!("ugly char", to_string_ugly(&'🦀')?);
    insta::assert_display_snapshot!("ugly unit", to_string_ugly(&())?);
    insta::assert_display_snapshot!("ugly some", to_string_ugly(&Some(0))?);
    insta::assert_display_snapshot!("ugly none", to_string_ugly(&None::<i32>)?);
    Ok(())
}

#[derive(Serialize)]
enum Enum {
    Unit,
    Newtype(i32),
    Tuple(i32, i32),
    Struct { field: i32 },
}

#[test]
fn ugly_variants() -> serde_kdl::Result {
    insta::assert_display_snapshot!("ugly unit variant", to_string_ugly(&Enum::Unit)?);
    insta::assert_display_snapshot!("ugly newtype variant", to_string_ugly(&Enum::Newtype(0))?);
    insta::assert_display_snapshot!("ugly tuple variant", to_string_ugly(&Enum::Tuple(0, 0))?);
    insta::assert_display_snapshot!(
        "ugly struct variant",
        to_string_ugly(&Enum::Struct { field: 0 })?
    );
    Ok(())
}

#[derive(Serialize)]
struct Unit;

#[derive(Serialize)]
struct Newtype(i32);

#[derive(Serialize)]
struct Tuple(i32, i32);

#[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct Struct {
    field: i32,
}

#[test]
fn ugly_structs() -> serde_kdl::Result {
    insta::assert_display_snapshot!("ugly unit struct", to_string_ugly(&Unit)?);
    insta::assert_display_snapshot!("ugly newtype struct", to_string_ugly(&Newtype(0))?);
    insta::assert_display_snapshot!("ugly tuple struct", to_string_ugly(&Tuple(0, 0))?);
    insta::assert_display_snapshot!("ugly struct", to_string_ugly(&Struct { field: 0 })?);
    Ok(())
}

type StringMap = BTreeMap<&'static str, u32>;
type ObjectMap = BTreeMap<Struct, Tuple>;

#[test]
fn ugly_hashmaps() -> serde_kdl::Result {
    insta::assert_display_snapshot!(
        "ugly string map",
        to_string_ugly::<StringMap>(&[("one", 1), ("two", 2)].into_iter().collect())?
    );
    insta::assert_display_snapshot!(
        "ugly object map",
        to_string_ugly::<ObjectMap>(
            &[
                (Struct { field: 1 }, Tuple(2, 3)),
                (Struct { field: 4 }, Tuple(5, 6))
            ]
            .into_iter()
            .collect()
        )?
    );
    Ok(())
}
