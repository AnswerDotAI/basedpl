# Speed cases

These BPL programs supply the native and browser speed tests. Each file owns one shared setup. `arrays.bpl` compares ordinary, exact and 32-bit arithmetic; the other files cover images, model objectives and derivatives, call overhead, structure, mapped functions and tables. The [record format](../reference/README.md#timed-cases) also supports independent result checks.

The three footer budgets are macOS on Apple silicon, Linux on CI's x86 runner, and wasm under Node on that runner. Native matrix products and decompositions use Accelerate on macOS and Rustymath's own kernels on Linux. `rustymath::timing::NATIVE_COLUMN` picks the native column. Machines with none, such as Linux on Arm, report each cost with `budget -`. Each speed test, native or browser, fails on a budget only on the CPU that the budgets were set for, `rustymath::timing::BUDGET_CPU`. On any other CPU, it reports every cost and marks each case still over its budget.

A copy is one read and one write of `n` 64-bit floats. Costs normalize to the machine's stored copy time, or one measured once. The `widths` section instead compares 32-bit time with the same expression in `exact`. A `static` section repeats its file's workloads with 32-bit integers and floats, still counting 64-bit copies: each float pass moves half the bytes and has twice as many vector lanes. Its expected cost is normally half the 64-bit cost.

A copy budget is the expected cost times the multiplier that `set_budgets` applies, at least 0.5, as basedpl's DEV.md gives. A width-ratio budget adds 0.25 to the expected fraction. A later expected cost needs a fuller understanding, not just a slower measurement. Preserve the comments recording each expected cost and later changes. One expected cost serves every platform until a specific platform difference is found.

On CI, wasm's 128-bit SIMD and lack of fused multiply-adds make vector arithmetic cost about twice its native AVX2 cost. Such wasm budgets are twice that expected cost. Scalar and memory-bound work has a smaller difference. CI's x86 runner is an AMD Ryzen 9 7950X3D. On macOS, Rustymath's `tests/speed.txt` header records Accelerate's matrix-product rates in flops per copy time; a case far from its class's rate records its own measurement.

`BASEDPL_COSTS=1 cargo test --test speed` logs all native costs. `bpl-tests speed_costs` measures the same sources through a worker. `node --no-liftoff --test tests/wasm-speed.mjs` measures them in a browser build. `set_budgets(expected, path='tests/speed/arrays.bpl')` updates only the selected footers.
