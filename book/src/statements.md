# Statements
The claims asserted by a POD are referred to as its *statements*. These statements introduce values and express relations between them, where the values may or may not be part of the same POD. The mechanism for referring to values in arbitrary PODs is furnished by *anchored keys*.

## Anchored keys
Rather than dealing with just keys, we introduce the notion of an *anchored key*, which is a pair consisting of an origin specifier and a key, i.e.

```
type AnchoredKey = (Origin, Key)
type Key = String
```

An *origin* is a triple consisting of a numeric identifier called the *origin ID*, a string called the *origin name* (omitted in the backend) and another numeric identifier called the *gadget ID*, which identifies the means by which the value corresponding to a given key is produced.

The origin ID is defined to be 0 for 'no origin' and 1 for 'self origin', otherwise it is the content ID[^content-id] of the POD to which it refers. The origin name is not cryptographically significant and is merely a convenience for the frontend.

The gadget ID takes on the values in the following table:

| Gadget ID | Meaning                                                                                   |
|-----------|-------------------------------------------------------------------------------------------|
| 0         | no gadget                                                                                 |
| 1         | `SignedPOD` gadget: The key-value pair was produced in the construction of a `SignedPOD`. |
| 2         | `MainPOD` gadget: The key-value pair was produced in the construction of a `MainPOD`.     |

For example, a gadget ID of 1 implies that the key-value pair in question was produced in the process of constructing a `SignedPOD`.

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
