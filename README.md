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

Note that this is not an official KDL document, and may be incompatible with
other Serde/KDL mappings. In this document, "serialization" refers to the
process of encoding a Serde value in KDL, and "deserialization" refers to the
reverse, decoding an SiK encoded document to the Serde data model.

### Goals

The main translation issue is that Serde is a value-based serialization
framework, while KDL is a node-based document language. Because of this
disconnect, one of the two sides of the bridge is going to feel somewhat
stunted. Here, and in other design decisions, we follow the lead of JiK and
XiK, in that we favor first the complete embedding of the Serde data model,
and then the ergonomics of the KDL microsyntax.

As such, like JiK, we introduce the idea of a "literal node" — that is, a
[KDL Node] with name `-`. This node serves the sole purpose of holding
[KDL Argument]s, [KDL Property]s, and (optionally) a [KDL Children Block].

Serde leaf values to a literal node with a single KDL Value, and serde compound
values map to a series of child nodes. Additionally, a KDL Value or KDL
Property may be used instead of a child leaf node, if (and only if) this does
not break KDL's ordering rules. (In practice, this means that Arguments must
appear before Properties, and that any data structure which should preserve
order must appear in a child block.) More specific rules are introduced in the
detailed mapping specification.

This results in a KDL microsyntax that feels fairly natural to read, write, and
edit, and takes advantage of most of the expressiveness that KDL offers. It is,
however, still a microsyntax, as not all valid KDL documents are necessarily
valid SiK. The main class of invalid KDL documents are those that utilize the
semantic difference between arguments/properties and child nodes, as SiK makes
no such distinction.

### Mappings

Serde's 14 primitive types map in the trivial manner: `bool` maps to
[KDL Boolean], all `iN`, `uN`, and `fN` map to [KDL Number], and `char` maps
to a [KDL String] of length 1. Similarly, a Serde `string` maps to a KDL
String as well.

Serde `byte array` (`[u8]`) maps to a Base64-encoded KDL String,
optionally with the standard `base64` type annotation.

Serde `option` follows serde-json's example and maps to either [KDL Null] for
a none value or the wrapped some value directly[^1].

Serde `unit` maps to KDL Null. Serde `unit_struct` serializes as if it were
`unit`[^4]. Serde `unit_variant` serializes as if it were `unit`, but with
a _mandatory_ [KDL Type Annotation] of the name of the variant.

Serde `newtype_struct` serializes as if it were its wrapped value[^4]. Serde
`newtype_variant` serializes as if it were `newtype_struct`, but with a
_mandatory_ KDL Type Annotation of the name of the variant.

Serde `seq` and `tuple` are both ordered collections which serialize as a KDL
Children Block containing literal (`-`) nodes. Order is significant, so these
MUST NOT be represented as KDL Values. `tuple_struct` serializes as if it were
`tuple`[^4]. `tuple_variant` serializes as if it were `tuple_struct`, but with
a _mandatory_ KDL Type Annotation of the name of the variant. If members of the
`tuple` are simple values (that is, serialize as a single KDL Value), they
MAY[^5] be uplifted and serialized as KDL Values of the `tuple` literal node,
rather than child literal nodes.

Serde `struct` is an unordered key-value pairing that serializes as a sequence
of child nodes, where the node name is the struct field key, and the node's
value is the serialized value. Serde `struct_variant` serializes as if it were
`struct`, but with a _mandatory_ KDL Type Annotation of the name of the
variant. As `struct` is unordered, any fields which have simple values MAY[^5]
be uplifted and serialized as KDL Properties of the `struct` node, rather than
child literal nodes. However, even though the fields _are_ unordered, they have
a "natural" order, and KDL Arguments MAY[^5] be used for fields as well. Each
KDL Argument maps to the next field in the natural order. All KDL Arguments
MUST occur before any KDL Properties; a KDL Argument after a KDL Property MUST
result in an error. Any duplicated struct fields (whether from a duplicated
name or a clash with an implicit name) SHOULD pass these through as duplicates
in the Serde model. If they are not passed through, then a KDL Argument
duplicating a named field MUST be an error, a KDL Property duplicating a child
node MUST also be an error, and a KDL Property duplicating a KDL Property MUST
be rightmost[^6]-Property wins, as specified by the KDL specification.

Serde `map` is tricky, due to the fact that KDL has no native representation
for value-value mappings. Note, though, that many serde data formats, such as 
serde-json, only support string-keyed maps. Note also that `map` is an
_ordered_ mapping, so it _must_ be represented by a KDL Children Block.

The only map supported without extensions is string-keyed maps. A string-keyed
map serializes the same as a `struct`, but MUST NOT use any Proprties. Map keys
SHOULD be quoted, even when not required by the KDL spec. Maps are inherently
ambiguous with a non-`map` mapping to the same data structure, of just `struct`
and/or `tuple`. The disambiguation is done via contextual type.

With extensions, other formats for map entries MAY be allowed. Mixing map entry
encoding styles MAY be accepted, but is _very strongly_ discouraged for KDL
authors and implementation serialization routines. Map entry encoding MAY be
required to match. If an implementation is capable, it SHOULD warn if map entry
encoding style does not match.

- If the key type is a string, the child node's name is the key string. If and
  only if the key is a valid KDL identifier, it may be a bare identifier;
  otherwise, it must be quoted (which is still valid KDL).
- If the key type is a leaf value, the child node is a literal node with name
  `-`, and has a property with name `key` which contains the map key; the map
  value is the node's single argument (if a leaf value) or a children block.
- The child node is a `tuple` of the key and value[^2].
- The child node is a `struct` with fields `key` and `value`.
- _AUTHOR NOTE:_ whoops, I think `- { - { key 0; value {}; } }` is potentially
  ambiguous with the shortened version `- { - key=0 { value {} } }`, as that
  could also be `- { - { key 0; value { value {} }; } }`... this is the only
  place in the spec that allows "flattening" a child struct into the parent
  in this fashion; a tuple `(u32, struct)` cannot (currently) be represented
  as `node 0 { field {} }`, but has to be `node 0 { - { field {} } }` (short)
  or `node { - 0; - { field {} }; }` (long). We should investigate if we can
  make this flattening generally acceptable for deserialization (and then)
  remove case 2 here as falling out of case 3 and 4 plus flattening, even if
  serialization will never emit such, due to rewriting expense.

### Standard Extensions

- When a node is known to be a literal node by contextual typing, a name other
  than `-` (e.g. `item` for a list `items`) MAY be allowed. An implementation
  MAY place whatever restriction on such replacement names it wishes, but a
  document using solely `-` literal nodes MUST successfully deserialize. (Note
  that this _does_ allow disallowing mixing of named and `-` literal nodes.)
- An implementation MAY allow mixing of KDL Arguments and KDL Properties. In
  order to remain complient with the KDL specification, it MUST treat the KDL
  identically to as if all KDL Arguments came before all KDL Properties. An
  implementation with this extension SHOULD never serialize such SiK, as this
  behavior is considered non-intuitive, and only exists as an extension for the
  purpose of compatibility. (A notable use case is implementing SiK on top of a
  KDL implementation that completely parses the document ahead of time, and
  properly does not expose the relative ordering of arguments and properties.)
- Type Annotations not requried by the spec MAY be allowed. If allowed, the
  type annotations MUST be required to match the expected deserialization type,
  and result in an error if mismatched. In the case multiple type annotations
  could be valid (e.g. multiple `newtype_struct` layers), an implementation
  that accepts a type annotation MUST document which types are allowed.
- Where this spec requires a Type Annotation, and the value is placed in a
  literal node, an implementation MAY instead accept the name of the literal
  node _instead of_ the type annotation. The type annotation then MUST follow
  the normal rules for optional type annotation.
- Alternative wrapper type serialization. If supported, these SHOULD be an
  option during serialization, and always accepted during deserialization.
  - Serde `option` MAY optionally be de/serialized as if it were a normal Serde
    `enum` variant of `None`/`Some`, rather than a special type.
  - Serde `unit_struct` MAY optionally be de/serialized as if it were a Serde
    `tuple_struct` containing a single `null`.
  - Serde `unit_variant` MAY optionally be de/serialized as if it were a Serde
    `tuple_variant` containing a single `null`.
  - Serde `newtype_struct` MAY optionally be de/serialized as if it were a Serde
    `tuple_struct` containing the wrapped value.
  - Serde `newtype_variant` MAY optionally be de/serialized as if it were a Serde
    `tuple_variant` containing the wrapped value.
- Alternative map entry serialization.
  - Map entries MAY be represented as a `tuple` of the key and value[^2].
  - Map entries MAY be represented as a `struct` with field `key` and `value`.
- For the specific case of a document root that is a Serde `struct` or `tuple`
  (or a data type that maps as such _without_ a mandatory type annotation),
  the literal node for the root MAY be omitted, instead placing its children as
  the (multiple) root-level nodes of the document.[^3]

If an implementation claims to implement the SiK spec, it MUST do one of:

- Declare that it implements "SiK," and faithfully implement the base spec.
  If it implements any extensions, it SHOULD instead do the following option.
- Declare that it implements "SiK with extensions," and faithfully implement
  the base spec plus extensions. Any SiK documented extensions SHOULD be
  documented, and any custom extensions MUST be documented. Base specification
  conformant SiK documents MUST always deserialize properly, but documents
  serialized with the implementation MAY require extensions to deserialize.
- Declare that it implements "SiK with variations." An implementation MAY
  deviate from the spec, but it MUST document exactly how it deviates.
  Additionally, such implementations are ENCOURAGED to submit their variation
  as an extension to this SiK document, and document their variation as
  mandatory use of the extension, for the purpose of interoperation.

### Special Extensions

These are specific special-case alternate mappings between specific Serde value
structures and KDL, that help translate idioms of either side more accurately.
These extensions are less likely to be supported than previous extensions, as
they often require extra context that is not easily available in streaming
contexts (such as the `serde` crate's API). An implementation which makes a
differentiation between Standard and Special Extensions SHOULD refer to the use
of Special Extensions as "Human SiK," as the purpose of these extensions is to
more closely map to how a human would write KDL if not under the restrictions
of the SiK microsyntax.

⚠ 🚧 This section is NON NORMATIVE and UNDER CONSTRUCTION 🚧 ⚠

- I'd like `struct { ...; struct { ... } }`<sub>serde</sub> to be able to map
  to more succinct KDL; it currently optimally (... properties only) maps as
  `- ... { - ... }`<sub>KDL</sub>; I'd like `- ... - ...`<sub>KDL</sub> to be
  valid. This would special case "tail `struct`". Also _very_ interesting is
  special casing it into `- ... { ... }`, to support meaningful properties... 👀
  - This might cause ambiguity in some cases? Is this resolvable in the serde
    streaming deserialization API without buffering? As an example, consider
    `- { key=0; { value { ... } }}`<sub>KDL</sub>, which might deserialize as
    `struct { key; value }` or `struct { key; struct {value}}`<sub>serde</sub>.
    This is especially meaningful for simplifying value-keyed map encoding.
  - It's worth calling out that this allows the more succinct value-struct map
    encoding of `- { - key=0 { ... } }`<sub>KDL</sub>, eliding the tail `value`
    field into the child block.
- Supporting an arbitrary number of arguments on a KDL Node followed by other
  data would require a special case for `struct { seq; ... }`<sub>serde</sub>.
  The author doesn't know how desirable this is; if you have a KDL document
  that benefits from structure, please share it! 
- ‼ As the author understands it, the official KDL Schema can't be meaningfully
  parsed as SiK. This seems _really_ unfortunate, and if it's possible to do so
  without compromising SiK's design goals, it would be very nice to support.
  Unfortunately, since the KDL Schema uses a meaningful split between node
  values, arguments, and children (as is a good source of expressiveness in a
  node-oriented language), this might require multiple special case mappings to
  capture properly. Note, though, that the mapped Serde data model _does not_
  have to match the derived de/serialization of a good Rust format for the
  schema; we can use custom `De`/`Serialize` implementations in order to map
  the desired Serde representaiton to the desired Rust data structure.
- A _really cool_ potential feature I want to see more of is generating schema
  from serde serialization and/or annotations where possible... 👀
- If there are idioms you'd like to see more succinct extension mappings for,
  PLEASE provide a PR! I want SiK to be as nice to work with as possible, no
  matter how complex an implementation has to be to support the entire list of
  extensions.

  [KDL Argument]: https://github.com/kdl-org/kdl/blob/main/SPEC.md#argument
  [KDL Document Language]: https://github.com/kdl-org/kdl
  [KDL Boolean]: https://github.com/kdl-org/kdl/blob/main/SPEC.md#boolean
  [KDL Children Block]: https://github.com/kdl-org/kdl/blob/main/SPEC.md#children-block
  [KDL Identifier]: https://github.com/kdl-org/kdl/blob/main/SPEC.md#identifier
  [KDL Node]: https://github.com/kdl-org/kdl/blob/main/SPEC.md#node
  [KDL Null]: https://github.com/kdl-org/kdl/blob/main/SPEC.md#null
  [KDL Number]: https://github.com/kdl-org/kdl/blob/main/SPEC.md#number
  [KDL Property]: https://github.com/kdl-org/kdl/blob/main/SPEC.md#property
  [KDL String]: https://github.com/kdl-org/kdl/blob/main/SPEC.md#string
  [KDL Type Annotation]: https://github.com/kdl-org/kdl/blob/main/SPEC.md#type-annotation
  [KDL Value]: https://github.com/kdl-org/kdl/blob/main/SPEC.md#value
  [Serde data model]: https://serde.rs/data-model.html

## Ideas for Extension?

Please either [make a new discussion](https://github.com/CAD97/serde-kdl/discussions/new) or
[drop your idea in the mega discussion](https://github.com/CAD97/serde-kdl/discussions/1);
we'd love to hear it!

## serde-kdl Versus SiK

serde-kdl, as a result of being primarly a one-enby project, doesn't fully
implement the SiK spec with all of the extensions. As of 0.0.0 development,
serde-kdl is incomplete and does not fully implement the base spec. For public
release, serde-kdl is intended to SiK with the extensions of:

- All alternative wrapper type serializations: `option` as `enum`,
  `unit_*` as `tuple_*`, and `newtype_*` as `tuple_*`.
- Value-value mapping via `struct { key; value; }` entries.
- Implied root node.

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

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

-----

[^1]:  Yes, this causes ambiguities with nested `option`, but this is a
conventional limitation of many serde data formats that you have to deal with
anyway if you want to support serializing to arbitrary formats. For a general
solution, you can use a helper like [`serde(with = unwrap_or_skip)`] to enforce
"absent `none`, transparent `some`", or [`serde(with = option_as_enum)`] to
enforce it to serialize as a `None`/`Some` variant in the serde data model.
For a SiK specific solution, many implementations support the extension to
serialize all `option` in your document as a plain `enum` instead.

[^4]:  This follows the expected behavior implemented in serde-json, and makes
the introduction of new nominal types (those distinguished solely by name, and
not by structure) completely transparent on the data side. If this is
undesirable, you can use `serde(with)` to serialize as a tuple instead of as a
newtype, or enable an extension to apply this transformation globally.

[^2]:  This is possible in any data format as [`serde(with = map_as_tuple_list)`].

[^3]:  This extension is the most "core" of the extensions, and likely expected
by anyone reading the KDL, as this removes a meaningless level of indentation
and allows the common practice of multiple root fields for data configuration.

[^5]:  An implementation MUST support all of these MAYs; the MAY refers to the
KDL document/data encoding, not the implementation.

[^6]:  "Rightmost" is a problematic specification in the face of non-LTR text.
This is an [upstream spec issue](https://github.com/kdl-org/kdl/issues/212).
It's reasonable to assume "rightmost" is intended to mean "later in the input
text," rather than lexically located to the right.

  [`serde(with = unwrap_or_skip)`]: <https://docs.rs/serde_with/1/serde_with/rust/unwrap_or_skip/index.html>
  [`serde(with = option_as_enum)`]: <https://github.com/jonasbb/serde_with/issues/365>
  [`serde(with = map_as_tuple_list)`]: <https://docs.rs/serde_with/1/serde_with/rust/map_as_tuple_list/index.html>
