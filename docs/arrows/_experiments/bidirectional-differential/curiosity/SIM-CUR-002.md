# SIM-CUR-002

**Classification:** `BD-COHERENT` *(corrected — was `B-ONLY-DRIFT`)*
**Runs:** A=3, B=3

This spec and its sibling are implemented in one function, and all six
reconstructions across the pair returned the same sentence: the union of both.

Originally filed as drift because neither spec was *attributed* individually.
That was the wrong test. The question this audit asks is whether a reader
recovers what the spec requires, and the sentence they returned contains it.
Extra content from a shared site is not drift; it is the shared site.

The attribution problem is real and is recorded — where two EARS share one
implementation with no seam, the method cannot say which it read. That limits
what a verdict means here. It does not make the content unrecovered.
