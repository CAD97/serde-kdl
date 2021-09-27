# serde-kdl

Yet another implementation of the [KDL Document Language], with a focus on
creating "obvious" mappings between KDL and the Serde data model, pulling from
precident set by serde-json, suitable for human inspection and modification.

## See also

- [kdl-org/kdl](https://github.com/kdl-org/kdl), the KDL specification
- [kdl-org/kdl-rs](https://github.com/kdl-org/kdl-rs), the Rust reference implementation
- [Lucretiel/kaydle](https://github.com/lucretiel/kaydle), another in-progress KDL ser/de

## Serde-in-KDL (SiK)

This is an informal description of how we map between the [Serde data model]
and the KDL language. This document serves a similar purpose to that of JiK
(JSON-in-KDL) and XiK (XML-in-KDL).

This is version 0.0.0-dev of SiK.

The main translation issue is that Serde is a value-based serialization
framework, while KDL is a node-based document language. We map serde leaf
values to [KDL Value]s, and compound values to [KDL Children Block]s. When
serializing a value in node position, we use a "literal node" as in JiK; that
is, a node with name `-`. Additionally, throughout SiK, any time a child node
is used, a [KDL Property] may be used instead if the value is a leaf KDL Value,
and similarly, a child literal node may be represented as a [KDL Argument].
This results in a KDL microsyntax that feels natural to read, write, and edit,
and takes advantage of the expressiveness that KDL offers. It is, however,
still a microsyntax, as not all valid KDL documents are necessarily valid SiK.
The main limitation is that it's not possible to mix properties and arguments.

Serde's 14 primitive types map in the trivial manner: `bool` maps to
[KDL Boolean], all `iN`, `uN`, and `fN` map to [KDL Number], and `char` maps
to a [KDL String] of length 1. Similarly, a Serde `string` maps to a KDL
String as well.

Serde `byte array` (`[u8]`) maps to a Base64-encoded KDL String,
optionally with the standard `base64` type annotation.

Serde `option` follows serde-json's example and maps to either [KDL Null] for
a none value or the wrapped some value directly<sup>[1]</sup>. However, it is
also supported to treat `option` as just variants as if it were a custom enum.

Serde `unit` maps to KDL Null. Serde `unit_struct` also maps to KDL Null,
optionally with a [KDL Type Annotation] of its name. Serde `unit_variant` maps
as a `unit` with mandatory KDL Type Annotation of the name of the variant.

Serde `newtype_struct` maps to just a serialization of its wrapped type,
optionally with a KDL Type Annotation of its name. Serde `newtype_variant`
maps as a `newtype_struct` with mandatory KDL Type Annotation of the name
of the variant.

Serde `seq` and `tuple` both map to a KDL Children Block containing literal
(`-`) nodes. `tuple_struct` also maps to the same, optionally with a KDL Type
Annotation of its name. `tuple_variant` maps as a `tuple_struct` with mandatory
KDL Type Annotation of the name of the variant.

Serde `struct` maps to a KDL Children Block containing nodes where the node
name is the name of the field, and nodes have a single value or children block.
Serde `struct_variant` maps as a `struct` with mandatory KDL Type Annotation of
the name of the variant.

Serde `map` is tricky, due to the fact that KDL has no native representation
for value-value mappings. (Note, though, that many serde data formats, such as 
serde-json, only support string-keyed maps.) In all cases, `map` maps to a KDL
Children Block, where its children are any applicable choice from this list:

- If the key type is a string, the child node's name is the key string. If and
  only if the key is a valid KDL identifier, it may be a bare identifier;
  otherwise, it must be quoted (which is still valid KDL).
- If the key type is a leaf value, the child node is a literal node with name
  `-`, and has a property with name `key` which contains the map key; the map
  value is the node's single argument (if a leaf value) or a children block.
- The child node is a `tuple` of the key and value<sup>[2]</sup>.
- The child node is a `struct` with fields `key` and `value`.

Mixing map child node types is allowed, but _very strongly_ discouraged. The
serialization formatters provided in this crate will never emit such SiK.
Alternative SiK implementations MAY require all map nodes have matching kind.

### Additional Convenience Features

If the root node is a compound type (not a primitive), its fields may be
directly placed as multiple root nodes in the KDL document instead.

  [KDL Argument]: <https://github.com/kdl-org/kdl/blob/main/SPEC.md#argument>
  [KDL Document Language]: <https://github.com/kdl-org/kdl>
  [KDL Boolean]: <https://github.com/kdl-org/kdl/blob/main/SPEC.md#boolean>
  [KDL Children Block]: <https://github.com/kdl-org/kdl/blob/main/SPEC.md#children-block>
  [KDL Identifier]: <https://github.com/kdl-org/kdl/blob/main/SPEC.md#identifier>
  [KDL Node]: <https://github.com/kdl-org/kdl/blob/main/SPEC.md#node>
  [KDL Null]: <https://github.com/kdl-org/kdl/blob/main/SPEC.md#null>
  [KDL Number]: <https://github.com/kdl-org/kdl/blob/main/SPEC.md#number>
  [KDL Property]: <https://github.com/kdl-org/kdl/blob/main/SPEC.md#property>
  [KDL String]: <https://github.com/kdl-org/kdl/blob/main/SPEC.md#string>
  [KDL Type Annotation]: <https://github.com/kdl-org/kdl/blob/main/SPEC.md#type-annotation>
  [KDL Value]: <https://github.com/kdl-org/kdl/blob/main/SPEC.md#value>
  [Serde data model]: <https://serde.rs/data-model.html>

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE/APACHE](LICENSE/APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE/MIT](LICENSE/MIT) or http://opensource.org/licenses/MIT)

at your option.

If you are a highly paid worker at any company that prioritises profit over
people, you can still use this crate. I simply wish you will unionise and push
back against the obsession for growth, control, and power that is rampant in
such workplaces. Please take a stand against the horrible working conditions
they inflict on your lesser paid colleagues, and more generally their
disrespect for the very human rights they typically claim to fight for.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

-----

[1] Yes, this causes ambiguities with nested `option`, but this is a
conventional limitation of many serde data formats that you have to deal with
anyway if you want to support serializing to arbitrary formats. For a general
solution, you can use a helper like [`serde(with = unwrap_or_skip)`] to enforce
"absent `none`, transparent `some`", or [`serde(with = option_as_enum)`] to
enforce it to serialize as a `None`/`Some` variant in the serde data model.

[2] This is possible in any data format as [`serde(with = map_as_tuple_list)`].

  [`serde(with = unwrap_or_skip)`]: <https://docs.rs/serde_with/1/serde_with/rust/unwrap_or_skip/index.html>
  [`serde(with = option_as_enum)`]: <https://github.com/jonasbb/serde_with/issues/365>
  [`serde(with = map_as_tuple_list)`]: <https://docs.rs/serde_with/1/serde_with/rust/map_as_tuple_list/index.html>
