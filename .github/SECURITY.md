# Security Policy

## Our Commitment

We fix security bugs before anything else, and we say what we found.

## What Counts as a Security Bug

Compote reads configuration files and environment variables and turns them into a typed value. A security bug
is one that could:

- Read a file or an environment variable other than the ones the caller named
- Show a configuration value in an error message, a panic, or `Debug` output where you would not expect it.
  Configuration often holds secrets
- Crash a program by panicking, overflowing the stack, or eating memory while parsing or merging a source
- Cause undefined behavior or otherwise break soundness
- Come from a known, unpatched bug in one of the parsers compote depends on

**Not security bugs:**

- Ordinary bugs that leave security alone
- Feature requests
- Slow code
- Mistakes in the docs
- A source doing what it says. Compote reads the sources you name and does not sandbox them, so a file you
  chose to read is as trusted as you made it
- Secrets you committed to your own configuration files

## Reporting a Vulnerability

Found one? Tell us right away.

### Reporting Process

**Do not** open a public issue. Report it through [GitHub's private vulnerability reporting][vulnerability-report].

Include:

- What the bug is
- How to reproduce it
- What it lets an attacker do
- Which versions it affects, if you know
- A fix, if you have one
- How to reach you with questions

### What to Expect

1. **Receipt.** We confirm within 24 hours that your report arrived
2. **Investigation.** We dig in and tell you what we find
3. **Fix.** Once we know the impact and have a patch, we ship it, agree the timing with you, announce it if it
   warrants an announcement, and credit you unless you would rather stay anonymous

### Response Timeline

- **24 hours** to confirm your report arrived
- **72 hours** for a first read on impact and severity
- **7 days** for full findings and a plan
- **30 days** to release the patch, longer if the fix is hard

## Disclosure Policy

1. **Confirm** the problem and work out which versions it affects
2. **Audit** the code for anything similar
3. **Prepare** fixes for every supported version
4. **Agree** the timing with you
5. **Release** the patches
6. **Publish** an advisory

## Supported Versions

| Version |    Support    |
|:-------:|:-------------:|
|  0.2.X  |   Supported   |
| > 0.2.X | Not Supported |

## Rules for Contributors

- Never commit a secret: no API keys, no tokens, no passwords
- Never let a malformed source panic. Parse failures belong in `Error`, and conversions that can fail belong in
  `TryFrom` rather than `as`
- Keep configuration values out of error messages, unless naming the value is the only way to say what went
  wrong
- Bound recursion when you add a format or a merge rule, so deeply nested input cannot exhaust the stack
- Keep `unsafe` out of the crate. Today the one exception is a test that sets an environment variable
- Turn on the fewest features a parser needs to do its job
- Keep dependencies current, and run `mise run audit` before a release

## Comments on this Policy

If you can improve this process, open a pull request or an issue.

## Contact

For anything urgent, message the maintainers on GitHub.

[vulnerability-report]: https://github.com/aaronmallen/compote/security/advisories/new
