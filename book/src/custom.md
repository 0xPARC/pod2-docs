# Custom statements and custom operations

Users of the POD system can introduce _custom predicates_ (previously called _custom statements_) to express complex logical relations not available in the built-in predicates.  Every custom predicate is defined as the conjunction (AND) or disjunction (OR) of a small number of other statements.

When a custom predicate is introduced in a MainPod, it becomes available for use in that POD and all PODs that inherit[^inherit] from it.

A custom predicate can be defined either _nonrecursively_ or _recursively_ (as part of a "batch"). A nonrecursive custom predicate is defined in terms of previously defined predicates (whether custom or native).  A "batch" of custom predicates can be defined _recursively_: the definition of any custom predicate in the batch can use both previously defined predicates and all the predicates in the batch.

## Custom predicates and their IDs

A custom predicate, like a built-in predicate, is identified by a _name_ on the front end and an _identifier_ on the back end.  In the non-recursive case, the back-end identifier is defined as a hash of the definition of the custom predicate.

### Recursively defined custom predicates



[^inherit]: What to call this?  One POD "inherits" from another?

