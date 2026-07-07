# JtV (Julia the Viper) — proof-of-concept

JtV is a Harvard-architecture language enforced at the grammar level, with
two grammatically-sealed sublanguages:

- **DATA** — an addition-only, total (provably-halting) expression
  language. No recursion, no loops, no conditionals branching on
  unevaluated control. Numbers are exact rationals.
- **CONTROL** — Turing-complete and imperative: assignment, `if`/`else`,
  `while`, `print`, blocks.

A **one-way bridge** (the "diode") lets CONTROL bind a *halted* DATA-expression
result into a control variable. DATA can never invoke control flow.

**Load-bearing property:** membership in the DATA grammar *is* the
termination certificate. If a fragment parses as a data-expression, it
terminates — so checking totality collapses to parsing. Because the
bridge is one-way, a data island contributes no non-termination to the
surrounding program: a data block is a **totality oracle** the control
logic may rely on.

This repo is a proof-of-concept of exactly that claim and nothing more.
It is not a language implementation, not a type checker, not an LSP. See
[LIMITATIONS](#limitations) below for what this specifically does *not*
claim.

## 60-second example

```
# examples/oracle.jtv
begin
  n = 5
  sum = 0
  i = 0
  while i < n do
    begin
      sum = sum plus i     # DATA: addition, guaranteed to halt
      i = i plus 1         # DATA: addition, guaranteed to halt
    end
  print(sum)
end
```

`i < n`, `sum plus i`, and `i plus 1` are DATA expressions — each one is
a totality oracle: it is guaranteed to halt the instant it's evaluated,
because addition and comparison over finite terms have nothing to loop
on. The surrounding `while` is CONTROL: openly Turing-complete. Nothing
in JtV proves *this loop* terminates — it happens to, because `i` counts
up to a fixed `n`, which is ordinary reasoning about this program, not a
guarantee the language gives you.

Running it prints `10`. See [LIMITATIONS](#limitations) for why that is
not the same as JtV proving the whole program halts.

Exact arithmetic, no float drift:

```
# examples/pure_data.jtv
1/3 plus 1/3 added_to 1/3
```

evaluates to exactly `1` — DATA numbers are `fractions.Fraction`, not
floats.

And the diode holding: [`examples/violation.jtv`](examples/violation.jtv)
tries to put an `if`/`then`/`else` on the right-hand side of an
assignment (i.e. inside DATA-grammar position) and is **rejected** —
that rejection is the diode working, not a bug.

## How to run

Requires Python 3.11+ (uses `fractions.Fraction` and PEP 604 union
types). No dependencies to run; `pytest` only for the test suite.

```sh
# Evaluate a standalone DATA expression:
python3 -m jtv.cli examples/pure_data.jtv --mode data      # -> 1

# Parse + execute a CONTROL program:
python3 -m jtv.cli examples/oracle.jtv --mode control      # -> 10

# Watch the diode reject a smuggling attempt:
python3 -m jtv.cli examples/violation.jtv --mode control   # -> DataParseError

# Run the test suite (the negative tests in tests/test_diode_negative.py
# are the heart of this POC):
python3 -m pip install pytest
PYTHONPATH=src python3 -m pytest tests/ -v
```

(`PYTHONPATH=src` is required because this POC deliberately has no
package build step — see LIMITATIONS.)

## Repo layout

- `GRAMMAR.ebnf` — full EBNF for DATA, CONTROL, and the bridge.
- `src/jtv/data_ast.py` / `data_parser.py` / `data_eval.py` — the DATA
  sublanguage: its own AST types, its own parser, its own evaluator.
- `src/jtv/control_ast.py` / `control_parser.py` / `control_eval.py` —
  the CONTROL sublanguage, likewise self-contained.
- `src/jtv/diode.py` — the structural validator; see its docstring for
  how the two enforcement layers (grammar separation + AST-construction
  validation) work together.
- `examples/` — `pure_data.jtv`, `oracle.jtv`, `violation.jtv`.
- `tests/test_diode_negative.py` — the negative tests: the actual point
  of this repository.

## Limitations

**This is a proof-of-concept of one specific claim, deliberately scoped
narrow. It is not a language implementation.**

### 1. Totality-oracle vs. whole-program-halting

JtV proves that *each individual DATA expression* halts — that's the
totality-oracle property, and it's what "membership in the grammar is
the termination certificate" means. **It is not a proof that a whole JtV
program halts.** CONTROL is openly Turing-complete (it has unrestricted
`while`), and a CONTROL program can loop forever exactly as any
imperative language can. Asserting whole-program halting would be the
specific error this README is written to avoid making. What JtV actually
buys you: whatever a CONTROL program's own logic can't get right (like
whether its loop counter is bounded), you at least never have to wonder
whether a *data block embedded in it* is the thing that hangs.

### 2. The green is only as wide as the data language

Addition-only is a maximally narrow total fragment, chosen specifically
because it makes the termination argument nearly free (there is
structurally nothing to loop on). Widening DATA toward recursion or
iteration — even something as modest as bounded `for`-loops or
user-defined functions — would force re-establishing totality from
scratch (e.g. via a termination measure, fuel, or a totality checker
distinct from the parser), and membership-checking would no longer
collapse to parsing. This POC does not attempt that; it demonstrates the
narrowest case where the collapse is clean.

### 3. Prior art

A prior-art / design-space search has now been done (see
[`dev-notes/2026-07-07-prior-art.md`](dev-notes/2026-07-07-prior-art.md)
for the full analysis and sources). The short version, stated honestly:

**Each of JtV's three ingredients is individually well-precedented. What
has no clear single predecessor is the specific *combination*, and above
all the *enforcement modality* — doing all of it purely structurally (two
grammars + two AST hierarchies, no type system, no termination checker,
no runtime guard), with the termination certificate being grammar
membership itself and the bridge being genuinely one-way.** JtV is best
understood as a minimalist demonstration of that unoccupied corner, not
as a new capability. The design space around it is crowded; the exact
point it stands on appears to be empty.

**How the neighbours obtain totality — the key contrast.** The goal of a
"total, non-Turing-complete sublanguage" is old (D.A. Turner framed the
*Security vs. Universality* choice in "Total Functional Programming",
2004). But every established neighbour derives totality from a **checker
or a type system**, never from grammar membership:

- **Dhall** — a total, deliberately non-Turing-complete config language.
  Its guarantee is explicitly type-system-mediated: *"if a Dhall
  configuration file type-checks then program evaluation/normalization
  will never fail."* Recursion is caught as a **type error**, not a parse
  failure; the only recursive structure (lists) is consumed solely via
  terminating primitives (`List/fold`). So Dhall reaches the same
  *guarantee* as JtV's DATA layer by the opposite *mechanism*. It also
  has no data/control split and no bridge — a Dhall file computes one
  pure value that a larger program consumes (a one-directional
  data-into-host relationship, the closest thing here to the "oracle"
  idea, but across a file boundary, not an embedded diode).
- **Starlark** and **CUE** — non-Turing-complete by **forbidding general
  recursion** (Starlark) and by abandoning functions/recursion for a
  constraint lattice (CUE). Restrict-and-evaluate, not grammar-membership.
- **Nickel** — the outlier that deliberately **keeps** Turing-completeness
  (it permits general recursion) and leans on gradual typing + runtime
  contracts instead. Not a totality neighbour at all; useful only to mark
  the far end of the axis.
- **Agda / Idris / Coq (Rocq)** and tools like **foetus** — totality via
  a dedicated **termination checker** run as a *separate post-parse
  phase* (structural/lexicographic descent, sized types, measures). This
  is exactly the machinery JtV's addition-only grammar is chosen to make
  unnecessary. (General termination analysis is Π⁰₂-hard, so every sound
  checker is necessarily incomplete — the standing reason totality
  usually needs a real analysis pass. JtV sidesteps that only by making
  its DATA grammar so weak there is structurally nothing to loop on.)

**On mechanism (1), grammar-membership = termination certificate.** The
closest genuine analogue is **parser-termination** work: *well-formed*
Parsing Expression Grammars, where a checked structural well-formedness
condition guarantees the *parser* halts (Blaudeau & Shankar, "A Verified
Packrat Parser Interpreter", CPP 2020). But that certifies termination of
*parsing the grammar*, not termination of *evaluating the object
language* — which is the gap JtV closes by choosing a grammar whose
membership coincides with evaluation-termination. Caveat, and it is a
real one: "membership is a termination certificate" holds **only** for a
suitably restricted (decidable-recognition) grammar class — a
Turing-powerful parser can itself diverge (cf. Turing-complete parsing,
MDPI *Mathematics* 2023). JtV's claim is therefore contingent on the
addition-only restriction, exactly as LIMITATIONS §2 already says.

**On mechanism (2), the one-way structural diode.** Stage/level
separation is thoroughly explored — two-level type theory (2LTT/CFTT,
Kovács), MetaML/MetaOCaml, Terra, staged λ-calculi (Feltman et al.,
ESOP 2016), Koka's divergence effect — but two things separate all of
them from JtV:
- They enforce the split with a **type system** (universe levels,
  modalities, effect rows, staging annotations), *not* two disjoint
  grammars validated at AST-construction time.
- Their bridges are **bidirectional or the totality runs the other way.**
  MetaML-style `quote`/`splice`/`lift` moves code *both* up and down. In
  2LTT the *total* layer is the compile-time meta-language while the
  runtime object layer has general recursion — the opposite polarity to
  JtV (where the embedded DATA is the total side and the CONTROL host is
  Turing-complete). **Terra** is the closest on "two genuinely separate
  AST hierarchies," but its object stage is not total and the meta/object
  relation is a performance-staging split, not a one-way total→control
  diode. A genuinely one-way structural bridge *from* a total sublanguage
  *into* a Turing-complete host appears unoccupied.

**On mechanism (3), the injectable guaranteed-halting oracle.** The idea
of a reliably-terminating sub-computation feeding a larger one exists —
staged property-based-testing generators evaluated entirely at
compile-time (Goldstein et al., "Fail Faster", 2025), Dhall's pure config
value, affine type-and-effect systems where capability-carrying terms
provably cannot diverge. In every case, though, halting is either an
*emergent* property of what happens to be statically known or a
*type/effect* discipline — not a block whose halting is guaranteed
*structurally* by the grammar it is written in. This is individually
JtV's weakest novelty claim; it is really a corollary of mechanisms (1)
and (2) rather than a separate contribution.

**Honest bottom line.** Nothing here is unprecedented in isolation, and
this repo does not claim otherwise. The total-vs-Turing-complete split is
Turner (2004); two-language structural separation is Terra/staging;
guaranteed-halting sub-computations appear in staged and effect-typed
settings. The contribution — such as it is for a proof-of-concept — is
the *modality*: collapsing totality-checking into parsing, and enforcing
a *one-way* DATA→CONTROL diode *purely structurally* (grammar + AST, no
types, no checker, no runtime guard), all at once. That specific point
looks empty on the map. Whether it is worth occupying is a separate
question this POC does not try to answer.

### Other explicit non-goals of this repo

- No package build/distribution step, no LSP, no richer data language
  than the grammar in `GRAMMAR.ebnf`, no CI beyond running the test
  suite locally. Adding any of these is out of scope for this POC.

## License

See [LICENSE](LICENSE) — placeholder, to be replaced.
