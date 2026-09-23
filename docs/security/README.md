# Security engineering

Security review focuses on bounded work and malformed inputs.

The core protects diff search with a work budget and input/output limits. Match search is bounded by candidate distance and work. Patch parsing checks syntax and declared lengths before application.

The FFI layer adds an ownership rule: every successful returned string must be released exactly once with diffloom_free_string.
