# CI timing audit, October 20, 2026

The largest opportunity is to stop running two independent native build pipelines for every code PR. Fix cache ownership next, and replace source installation of CI tools with pinned binaries or a small tool cache. Keep Windows, macOS ARM64, Linux, legacy vault compatibility, strict Clippy, and release signature verification.

No workflows, repository settings, cache entries, or external services were changed. This report uses read-only GitHub CLI queries. No local application builds or new Actions runs were started. Blacksmith CLI was not needed and no Blacksmith service was invoked.

## Scope and measurement

- Retrieved all 739 available workflow run records, spanning January 30 to October 19, 2026.
- Retrieved job and step metadata for 228 runs, comprising 443 jobs. This includes all 213 executed, non-Dependabot runs from September 5 onward and 15 older comparison runs. Skipped workflows and GitHub-managed Dependabot update jobs are excluded from detailed timing totals.
- Read 11 detailed native job logs, all five current workflow definitions, cache inventory, and artifact metadata.
- Current baseline starts at October 18, 16:53:37 UTC, when the three-platform Blacksmith matrix appeared. It contains six successful CI runs, four successful package runs, one successful release, eight audits, and five dependency reviews, plus one cancelled CI/package pair. This is a small transition-day sample, not a stable month-long benchmark.
- Local checkout is `8234f1c`, the current main commit at inspection.

Wall time below is successful runs' `updated_at - run_started_at`, including scheduling and finalization. Runner-minutes sum each job's `completed_at - started_at`, including setup and cleanup. Parallel jobs add runner-minutes without adding the same amount to feedback time. These are elapsed runner-minutes, not Blacksmith invoice minutes or dollar costs. GitHub timestamps have second-level precision. Reported percentiles would be misleading with this sample, so medians and ranges are used.

## Current baseline

| Workflow | Successful sample | Median wall time | Observed wall range | Runner-minutes across current sample, including cancelled runs |
| --- | ---: | ---: | ---: | ---: |
| CI | 6 | 8m 59s | 3m 51s to 9m 35s | 128.7 across 7 runs |
| Native GPUI packages | 4 | 8m 10s | 2m 18s to 9m 02s | 78.7 across 5 runs |
| Release | 1 | 9m 10s | One observation | 21.1 |
| Security Audit | 8 | 1m 41s | 1m 35s to 1m 59s | 12.7 |
| Dependency Review | 5 | 16s | 15s to 16s | 0.6 |

Current CI native job medians are 8m 10s on Windows, 7m 38s on Linux, and about 4m 44s on macOS. Release tooling takes 10 to 12 seconds in these successful CI runs. The median delay to the first job is about 8 seconds for CI and packaging, so compilation and duplicate execution deserve attention before runner capacity.

The latest code PR is a useful concrete baseline. [CI](https://github.com/PeeeBrain/latch/actions/runs/39149362818) used 20.82 runner-minutes, [packaging](https://github.com/PeeeBrain/latch/actions/runs/39149362870) used 19.28, [audit](https://github.com/PeeeBrain/latch/actions/runs/39149362809) used 1.67, and [dependency review](https://github.com/PeeeBrain/latch/actions/runs/39149362801) used 0.12. Total execution was **41.88 runner-minutes**, with **8m 27s** until the slowest workflow finished.

## Findings, ordered by payoff

### 1. CI and packaging duplicate the expensive work

Both `ci.yml` and `native-package.yml` trigger on `crates/**`, root manifests, lockfile, and Rust toolchain changes. Each starts a three-platform matrix. Both run default core/desktop tests and compile `latch-desktop --release`. They run concurrently, so a shared cache cannot transfer work that has not finished yet.

On the latest PR, the package workflow spent the following time repeating work covered by CI:

| Platform | Package native test step | Package release compilation | Repeated total |
| --- | ---: | ---: | ---: |
| Windows | 125s | 213s | 338s |
| Linux | 103s | 213s | 316s |
| macOS ARM64 | 82s | 112s | 194s |
| Total | 310s | 538s | **848s, or 14.13 runner-minutes** |

This is a measured overlap budget, not a promise that every second disappears after restructuring. Cache state, target unification, and job placement affect the result. Installer generation itself took only 15s, 10s, and 24s respectively.

Recommended first change: keep CI's existing native checks, tests, release executables, and artifacts. Restrict automatic package PR runs to packaging configuration, release scripts, installer resources, and packaging workflow changes. Keep `workflow_dispatch` for any PR needing installers and retain tag releases. Define the resource paths from the packager configuration before narrowing the filter. This preserves the repository's CI-only executable review process.

For an ordinary code PR like the latest one, skipping the independent package workflow would remove **19.28 runner-minutes, 46% of its total execution**. Feedback would still take about 8m 27s with unchanged CI because the workflows already run in parallel.

If installers must remain automatic on every code PR, run packaging after the native checks/build in the same platform job, using that executable and target directory. This avoids a second checkout, test compilation, and release compilation. Expect some added tail time in that job for tool setup and packaging. Avoid a separate dependent job that recompiles after downloading only the executable.

### 2. Cache scope and competing writers cause repeated cold starts

The cache action works when a usable cache exists. The [warm package run](https://github.com/PeeeBrain/latch/actions/runs/39148474836) finished in 2m 18s. Its Linux job restored the exact key and finished in 90s, compared with 422s for the latest Linux package job. The corresponding [CI run](https://github.com/PeeeBrain/latch/actions/runs/39148474819) finished in 3m 51s. These observations have different commits and cannot establish a controlled speedup.

The latest PR's six native jobs all started without a cache in the inspected logs. CI and packaging used identical keys for each OS. Packaging finished first and uploaded its cache; all three CI jobs then reported `Failed to save: Unable to reserve cache ... another job may be creating this cache.` An overlapping main CI and release Linux job also collided.

Sharing a key is intentional in the YAML, but the jobs populate different contents. Packaging does not run `cargo check` or legacy compatibility checks. Since caches are immutable, its earlier save can leave later CI runs without those artifacts. A cache hit is therefore not proof that every CI command is warm. [GitHub cache behavior](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching), [rust-cache behavior](https://github.com/Swatinem/rust-cache).

At inspection, the cache API listed six entries totaling **8.75 GB decimal**. Four entries, totaling 5.29 GB, belonged to PR merge refs. Three entries, totaling 4.26 GB, targeted the removed Intel macOS matrix. No Linux cache appeared in that snapshot, despite successful upload messages in the logs. This is consistent with storage churn or backend retention differences, but the snapshot does not prove the cause. The configured storage limit was not established. GitHub's default limit is 10 GB, and PR-created caches cannot seed main or sibling PRs. [GitHub scope and eviction rules](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching).

Plan:

1. Give each cache key one intended writer for each OS, architecture, and build role. Prefer the full native CI job as the writer; package consumers should not win the same immutable key with less complete content. A unified native/package job resolves this naturally.
2. Keep main CI as the trusted cache seed. Let the existing main run complete before evaluating warm-cache behavior; the transition-day burst launched several PR, main, and tag jobs before seeding finished.
3. Preserve PR-local cache writes initially if repeated edits benefit from them. If inventory shows eviction pressure, make package readers restore-only and then consider main-only writes for dependency caches. Do not disable all PR writes before verifying main caches exist for all platforms.
4. Inventory obsolete Intel and closed-PR caches, then remove only confirmed unused entries during implementation. No entries were deleted in this audit. Avoid increasing paid cache capacity as the first fix.
5. Record cache hit, selected key, archive size, restore/save time, and actual compilation time. Check that unrelated preinstalled Rust toolchains are not fragmenting the key when runner images change. The logs include both the pinned compiler and a preinstalled compiler in the environment hash.

Use separate role keys only if separate compilation roles remain. Splitting every combination into a full target cache would consume more storage. `rust-cache` normally keeps dependency artifacts and removes workspace outputs, so `shared-key` is not an artifact handoff. [Action documentation](https://github.com/Swatinem/rust-cache).

Do not schedule frequent native builds solely to keep caches warm. GitHub can expire unused caches after seven days, so this relatively intermittent repository will sometimes pay a legitimate cold-start cost. Buying repeated warming runs can cost more than the work it avoids. [Cache retention](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching).

### 3. Security Audit spends most of its run compiling its own tool

Across the eight current audits, `Install Cargo audit` took a median **85s**, while `Run Cargo audit` took **4s in every run**. The workflow runs `cargo install cargo-audit --locked` without a dedicated tool cache. The latest scheduled [audit](https://github.com/PeeeBrain/latch/actions/runs/39225391832) is an example.

Install a pinned prebuilt `cargo-audit` binary with checksum verification instead. `taiki-e/install-action` explicitly supports it; pin the action and tool and disable source fallback for this supported tool. Keep the actual audit and its fresh advisory database. [Supported tools](https://github.com/taiki-e/install-action/blob/main/TOOLS.md), [installation options](https://github.com/taiki-e/install-action).

An initial target is **20 to 35 seconds per audit**, saving approximately 65 to 80 seconds per run, subject to measurement. The historical executed window contains 62 audits, so the frequency makes this worthwhile. Add concurrency cancellation for superseded PR audit runs while retaining push, weekly schedule, and manual coverage.

### 4. Packaging repeatedly installs its tool from source

The latest package run installed pinned `cargo-packager 0.11.8` in **64s on Windows, 40s on Linux, and 31s on macOS**, totaling 2.25 runner-minutes. Some warm runs skip installation because the binary survives in the general Cargo cache, but this depends on which competing job wrote the cache.

Give the tool a small, independent cache keyed by OS, architecture, tool version, and the relevant compiler identity. Restore the executable and Cargo installation metadata together, then verify `cargo packager --version`. An existing cache action supports versioned Cargo tool installation. Check for official prebuilt assets for this exact version and platform before choosing a binary installation route. Do not assume that the audit install action supports this tool through its no-fallback manifest. [Cargo tool cache action](https://github.com/taiki-e/cache-cargo-install-action), [packager project](https://github.com/crabnebula-dev/cargo-packager).

This improves cold package/release runs and prevents a change to Latch's lockfile from forcing a rebuild of an unchanged packaging tool. Tool-cache savings overlap with package-job removal and must not be added to that 46% estimate.

### 5. Some triggers buy more native work than the change requires

`ci.yml` triggers on every file under `scripts/**` and `.github/workflows/**`, then launches all native jobs and release contract tests. A release-note script change or audit workflow adjustment can therefore trigger the full native matrix. Conversely, release tooling is repeated on source-only PRs, although its 10 to 12 seconds are a low priority.

Use one inexpensive change-classification job and stable final required status. Run release tooling for relevant scripts and release configuration; native validation for Rust/toolchain/build resources; package checks for installer configuration/resources. Include a workflow's own file in its validation paths. Keep an always-running dependency review or otherwise preserve its required status behavior. Validate that job-level skips do not leave branch protection waiting on a missing workflow.

Keep the existing weekly, grouped Dependabot configuration. It already limits routine update PRs. Do not reintroduce individual PRs per dependency.

CI and package PR concurrency cancellation already exists. The cancelled pair in the current sample nevertheless executed **35.05 runner-minutes**, including jobs that completed before cancellation. Main CI uses a unique run ID in its concurrency group, so overlapping main pushes are never superseded. Consider cancelling stale main validation by ref if every intermediate commit does not need an artifact. Keep release cancellation disabled. The data does not justify claiming a large steady-state saving from this change alone.

### 6. Expensive packaging can complete even when compatibility checks fail

On the October 18 jsonwebtoken update, [CI failed historical compatibility tests](https://github.com/PeeeBrain/latch/actions/runs/39145534917), while the independent [package workflow succeeded](https://github.com/PeeeBrain/latch/actions/runs/39145534983), consuming **90.97 runner-minutes** on its then-four-platform matrix. This is historical evidence of unnecessary downstream work, not a forecast for today's smaller matrix.

Running optional packaging after the required native checks prevents this. A cheap Linux core/compatibility preflight may also catch portable failures before the full matrix, but it adds a serial dependency and extra setup on every healthy run. Add that gate only if failure frequency after the first optimizations justifies it. Preserve all platform checks and legacy vault tests.

## Workflows that are already economical

- The latest Linux CI desktop-test step took 88s, but the test binary itself ran in 1.53s after about 86s of compilation. Core default tests ran in 5.35s and legacy tests in 10.51s. Focus on compilation reuse before adding test sharding or a new test runner.
- Formatting and strict Clippy steps are short after compilation. Do not count removing `cargo check` as saving its entire duration: Clippy still needs to type-check the targets. Retain the prescribed checks in the initial optimization.
- Dependency Review runs in 15 to 16 seconds currently and consumes about seven runner-seconds per run. Keep it. Its comparison of dependency changes and Cargo's advisory scan provide different coverage.
- Release contract tests take 10 to 12 seconds including setup. Preserve them; narrowing their trigger is sufficient.
- The successful release spent only six runner-seconds validating and 22 publishing. Its 9m 10s wall time is driven by native packaging. Preserve signing, signature verification, tag/version/ancestry checks, and complete-asset verification.
- The removed Intel macOS matrix was an old bottleneck. The October 18 pre-streamlining [main CI run](https://github.com/PeeeBrain/latch/actions/runs/39145440999) spent 46m 37s in that job. Current workflows already removed it, so claiming that removal as a new saving would be wrong.
- There were 116 skipped `opencode` workflows since September 5. They add run-list noise, but no executed job cost is established for them. They are absent from the current workflow files.

## Implementation plan and acceptance criteria

| Stage | Change | Expected result | Validation |
| --- | --- | --- | --- |
| 1 | Restrict automatic packaging to package-related changes; retain manual installers and CI preview executables | About 19 runner-minutes removed on the latest ordinary code PR equivalent | Source-only PR still runs all required native checks and uploads all three executables; resource/package changes still validate installers |
| 1 | Pin prebuilt cargo-audit and add PR concurrency | Audit target of 20 to 35 seconds | Same lockfile/advisory policy gives the same pass/fail result |
| 2 | Establish cache writers, warm main, inventory obsolete entries, independently cache packager | Fewer cold starts and reserve collisions; recover much of the 65 to 80 seconds of packager install overhead on Windows/Linux | Compare identical-SHA cold and warm CI runs; inspect cache contents and restore/compile times on all three platforms |
| 3 | Classify changes inside CI; ensure stable final required status; optionally cancel stale main runs | Avoid native matrices for script-only or unrelated workflow edits | Check docs-only, release-script-only, Rust, lockfile, workflow, and installer-resource cases against branch protection |
| 4 | For PRs needing installers, reuse the same native job's tests and release build | Recover the measured 14.13 runner-minute duplicate-work budget on a cold PR equivalent | Same-SHA artifacts have expected platform/architecture; installer and updater contracts still pass |

For a second, optional policy change, routine PRs can run all current checks/tests without optimized executables, with executable builds requested manually or for review-ready changes. On the latest PR, removing packaging and CI release-build steps would leave a simple arithmetic budget of **13.46 runner-minutes**, about 68% below 41.88. Subtracting the Windows release-build step gives a feedback estimate near five minutes for that sample. These are counterfactual estimates, not measured outcomes, and would require an explicit replacement for automatic review executables. They are not the recommended first stage.

A practical follow-up target is 3 to 5 minutes for routine validation and 40% to 60% less execution on code PRs that do not need installers. The observed warm CI run reached 3m 51s even with release executables. Cache fixes and removal of duplicate packaging overlap, so do not sum their advertised savings. Measure at least ten comparable PR runs before treating the target as achieved.

Run actionlint on workflow changes. Follow AGENTS.md's required formatting, compilation, strict Clippy, and default/legacy/desktop test checks before creating an implementation PR. Use CI-produced artifacts to verify native launch and packaging; do not run local builds or build jobs through act.

## Historical context and limits

Across the September 5 onward detailed window, CI used 881.38 runner-minutes, native packaging 579.48, Security Audit 90.58, Dependency Review 12.27, the current Release workflow 80.02, and retired Release Build 16.73. These include failed and cancelled runs. CI had 22 failures out of 71 runs, and audit had 15 out of 62. Four failed native jobs came from one legacy compatibility failure; 19 failures occurred in the older combined backend check step. They are not evidence of flaky tests, and retries should not be added without diagnosis.

The Tauri-to-GPUI migration and runner/matrix changes make a pooled all-history median unsuitable for forecasting current performance. No Blacksmith billing export, runner price comparison, complete older job history, or controlled before/after experiment was collected. Exact cost savings remain unverified. No paid cache increase, runner upgrade, Codesmith, or other paid service is part of this plan.

Read-only collection used `gh api` for workflow runs, jobs with `filter=all`, individual job logs, cache usage/listing, and artifacts. All selected runs reported attempt 1. The accompanying CSV contains current-period job and step timings for review. Raw GitHub API responses and inspected logs were retained locally in the task's temporary audit directory.
