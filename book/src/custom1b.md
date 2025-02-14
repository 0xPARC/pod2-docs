# Custom operations (or: how to define a custom predicate), version 1, written again

[All constant integers in this spec are determined by circuit size and subject to change.]

A _custom predicate_ can be defined in one of two ways:

- Directly, as either the AND or OR of two pre-existing predicates, or
- Recursively, in a _batch_ of up to 10 custom predicates.  Each custom predicate in the batch is either the AND or OR of five predicates which are either pre-existing, or defined in the same batch.

[Note: We could potentially allow the AND or OR of, say, two predicates instead of five.  To make this work, we might need to have access to pod ID as a virtual key, see github issue #60]

## Arguments of custom predicates

The definition of a custom predicate might also be called an _operation_ or _deduction rule_.  It includes two (or, potentially, say, five) input statements.  The arguments to the input statements are decomposed as (origin, key) pairs: if statements are allowed to have arity at most 4, then the input statements in a deduction rule will have at most 8 arguments (4 origins and 4 keys).  The same holds for the output statement.

Each argument (origin or key) to an input statement is either a wildcard or a literal.  In the backend, the wildcard arguments will be identified as *1, *2, *3, ....

## Examples

See [examples](./customexample.md)

## Hashing and predicate IDs

Each custom predicate is assigned a cryptographic ID as follows:

If it is defined directly: its ID is simply a zk-friendly hash of its definition.  The definition is serialized as it will appear in circuit (see below) and hashed.

If it is defined in a batch: The definitions of all statements in the batch are laid out consecutively (see [examples](./customexample.md)) and hashed.

## How to prove an application of an operation

A POD contains a "tabular proof", in which each row includes a "statement" and a "reason".  The "reason" is everything the circuit needs as a witness to verify the statement.

For a custom statement, the "reason" includes the following witnesses and verifications:
- the definition of the statement, serialized (see [examples](./customexample.md))
  - if the statement is part of a batch, the definition of the full batch, serialized
- verify that the hash of the definition is the statement ID
- the definition will have some number of "wildcards" (*1, *2, ...) as arguments to input statements; a value for each wildcard must be provided as a witness (each will be either an origin ID or key)
- the circuit must substitute the claimed values for the wildcards, and the resulting statements (true statements with origins and keys) will appear as witnesses
- the circuit must verify that all the input statements (with origins and keys) appear in the previous statements (in higher rows of the table)
- the circuit also substitutes the claimed values for the wildcards in the output statement, and verifies that it matches the claimed output statement
