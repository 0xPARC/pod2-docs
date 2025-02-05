# Statements
The claims asserted by a POD are referred to as its *statements*. These statements introduce values and express relations between them, where the values may or may not be part of the same POD. The mechanism for referring to values in arbitrary PODs is furnished by *anchored keys*.



## Statement types
A statement is a code (or, in the frontend, string identifier) followed by 0 or more arguments. These arguments may consist of up to three anchored keys and up to one POD value.

The following table summarises the natively-supported statements, where we write `value_of(ak)` for 'the value anchored key `ak` maps to', which is of type `PODValue`, and `key_of(ak)` for the key part of `ak`:

| Code | Identifier  | Args                | Meaning                                                           |
|------|-------------|---------------------|-------------------------------------------------------------------|
| 0    | `None`      |                     | no statement (useful for padding)                                 |
| 1    | `ValueOf`   | `ak`, `value`       | `value_of(ak) = value`                                            |
| 2    | `Eq`        | `ak1`, `ak2`        | `value_of(ak1) = value_of(ak2)`                                   |
| 3    | `NEq`       | `ak1`, `ak2`        | `value_of(ak1) != value_of(ak2)`                                  |
| 4    | `Gt`        | `ak1`, `ak2`        | `value_of(ak1) > value_of(ak2)`                                   |
| 5    | `LEq`       | `ak1`, `ak2`        | `value_of(ak1) <= value_of(ak2)`                                  |
| 6    | `Contains`  | `ak1`, `ak2`        | `(key_of(ak2), value_of(ak2)) ∈ value_of(ak1)` (Merkle inclusion) |
| 7    | `Sintains`  | `ak1`, `ak2`        | `(key_of(ak2), value_of(ak2)) ∉ value_of(ak1)` (Merkle exclusion) |
| 8    | `SumOf`     | `ak1`, `ak2`, `ak3` | `value_of(ak1) = value_of(ak2) + value_of(ak3)`                   |
| 9    | `ProductOf` | `ak1`, `ak2`, `ak3` | `value_of(ak1) = value_of(ak2) * value_of(ak3)`                   |
| 10   | `MaxOf`     | `ak1`, `ak2`, `ak3` | `value_of(ak1) = max(value_of(ak2), value_of(ak3))`               |

[^content-id]: <font color="red">TODO</font> Refer to this when it is documented.
