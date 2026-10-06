# A covered match can return the wrong value

The learner predicted that an exhaustive match with the wrong Duration string would compile but fail its assertion at execution. They ran the scratch program, explained the reported left value ("distance") as as_str's actual return and the right value ("duration") as the expectation, repaired the mapping, recompiled, and supplied a passing execution. They explained exhaustiveness as coverage of all cases; feedback clarified that the assertion checks only the specific Duration mapping.

Evidence: learner-supplied assertion panic, interpretation, and successful recompiled execution in lesson 0001. The core predict–compile–fail–repair exercise is complete. The teacher supplied assert_eq! syntax, so independent test design and later recall remain pending. A panic was introduced only as the observed assertion failure; Result handling has not been taught.
