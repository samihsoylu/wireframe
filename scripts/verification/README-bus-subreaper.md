# Owned private-bus subreaper

This is test-runtime source, not product code or an operational grant. It is
intended to prevent owned D-Bus/portal descendants becoming unreaped zombies
under a container PID1 that only runs sleep. The supervisor must be an ancestor
of dbus-run-session; the existing hold process is its sibling and cannot do this.
No code runs on import. Production CLI requires the fixed verification Root and
real/effective/saved UID/GID65534. It uses Python/libc already in the environment.

The supervisor enables and verifies PR_SET_CHILD_SUBREAPER before forking. Its
child blocks until an exclusive launch record captures PID/start ticks and all
UID/GID fields. The later adapter pins the real post-exec argv and parent chain.
For each waitable child it records its still-present zombie identity using
waitid(WNOWAIT), then consumes that exact child's waitpid status. It never treats
Z as absent and sends no signals. Imported descendants are only kernel-adopted
children of this supervisor, not unrelated container processes.

Wait records retain any captured identity even when a check fails, alongside named `identityChecks`. A naturally exited adopted child may have different effective/saved/filesystem IDs only when its kernel wait UID, real UID/GID, current supervisor parent, PID and zombie state match. Such a row is explicitly `adoptedReapOnlyCredentialVariant`; every row sets `signalAuthorityFromWaitRecord=false`. The original direct child still requires every UID/GID slot and its launch start-time pin. Missing/unreadable or invalid real identity remains a fatal nonzero result. These records authorize no live-process signal.

The direct child exit is transparent: ordinary exits retain their number;
signals retain the signed child exit/raw status and use outer shell encoding
128+signal. Adopted child raw nonzero/signals are retained separately; outer0
means direct bus exit0 plus complete reaping, not that all children exited0 or
that native/service/archive acceptance passed. The parent must still consume the
real outer terminal result and run unchanged strict PID-absence/archive checks.

A valid original bus-stop-request, direct child exit or fatal validation error starts the bounded
shutdown deadline (default15s, maximum120s). Deadline failure returns124, records
remaining direct children and does not claim reaping/absence. No signals or force
cleanup occur. Parent retains failed scope and owns any separately authorized
normal cleanup/container closure. Other infrastructure failures are nonzero;
raw direct-child nonzero is retained when available. The four new private files
are exclusive0600, with a <=1MiB wait log and <=256 retained wait records. Old
files are never overwritten. Account for the helper and records in the existing
capacity budget; no budget increase is implied.

## Separately authorized Linux tests

Do not run these during source-only preparation. Use a dedicated Linux Python
process as UID/GID65534, with no existing children and an already created canonical
absolute artifact directory owned65534:65534 mode0700. Set both:

- `WIREFRAME_BUS_SUBREAPER_TEST_AUTHORIZED=1`
- `WIREFRAME_BUS_SUBREAPER_TEST_ARTIFACTS=/ABS/PARENT-OWNED-TEST-ARTIFACTS`

Then invoke the staged `test_bus_subreaper.py` using assertions-enabled Python
with `-I -B`. The six kernel fixtures cover double-fork adoption/raw exit23/actual PID absence,
direct exit7, raw self-SIGTERM/explicit143 encoding, bounded124 timeout with live
children honestly reported, failed exec127, and wrong-UID refusal before fork.
All fixture children self-exit; tests send no cleanup signals and never use
SIGKILL. The dedicated test parent acts as an outer subreaper solely to consume
finite children after the intentional timeout. Per-test mkdtemp directories and
all raw launch/wait/result/parent observations remain for archive/review; no test
artifact cleanup is performed. Skipped tests are not runtime proof.

## Adapter and shutdown integration

A proposed adapted direct_guest.py and diff are delivered separately. Stage/pin
this supervisor as `Root/bus_subreaper.py` only in a future source-reviewed case;
refresh the exact helper/adapter hashes and staged-file/capacity accounting. The
product, policy, guest_base, session_closure and existing H7/P6 files are unchanged.
The adapter keeps desktop.pid for the foreground tool-managed supervisor, pins a
new dbus-run-session child under it, and locates the daemon under that actual
child. Startup checks and a separate bus-session-ancestry.json bind the original
hold process PPID to that same child while both pinned identities are live. This
receipt is retained; later identity checks do not freeze PPID across legitimate
adoption during shutdown. Every recorded process remains subject to strict absence before archive.

After ordinary app and monitor/viewer/VNC/Openbox closure, request the original
bus stop. Obtain the new `bus-child-closure` record before any remaining owned
portal SIGTERM: it verifies the original bus child's genuine raw exit0. If a
pinned portal is still live, the existing authorized `stop-services` operation
can run while the supervisor remains alive to reap it; no D-Bus calls occur after
bus closure. Parent then consumes the original supervisor outer handle to real
terminal0 and calls `bus-supervisor-closure`, followed by unchanged strict service
closure and archive. Do not wait for final supervisor exit before requesting
needed portal stops: that could deadlock against the reaping deadline. A timeout
or nonzero child preserves failure and cannot authorize archive.

No Linux test, process fixture, VM/Docker action, staging or UI was executed when
these sources were prepared. Syntax compilation alone does not prove adoption,
reaping or actual service closure.

## Actual private session-bus check

The additional named test `BusSubreaperTests.test_real_dbus_session_daemon_is_adopted_reaped_and_absent` wraps the installed `dbus-run-session` and a finite Python holder. It records live holder/bus/daemon ancestry before ordinary stop, then requires the original bus child and adopted daemon to be genuinely reaped and absent. The daemon raw status is retained rather than assumed zero. Run only this named delta after its review; the six unchanged kernel fixtures already have retained acceptance.

On 1 November 2026 this named test passed on Linux ARM64/UID65534, with actual bus/daemon raw waits0, all hierarchy PIDs absent and normal container/VM closure. It exercised no portal or product/UI. Source review2393934b and independent retained-runtime review0ada851c retain its scope. The late portal-SIGTERM adapter guard now has independent source acceptance and nine passing closed mocked scenarios. The full native service/archive case remains open.

## Credential correction and actual kernel evaluation — 2 November 2026

Core `09d24288…` and focused fixture `3665b8b5…` passed source review `b42dd2a5…` after correcting a fixture-only two-link stop request. The final fixture uses Linux atomic no-replace rename and preserves the core single-link check. Ten real credential cases passed with no skips in a dedicated isolated Root library test (`expected_uid/gid=0`). The seven unchanged regressions then passed separately as UID/GID65534 on the same changed core, including the real D-Bus session. The original seven tests are rerun here because the core changed.

The Root library suite requires its separate explicit `WIREFRAME_BUS_SUBREAPER_CREDENTIAL_TEST_AUTHORIZED=1` and canonical Root-owned0700 `WIREFRAME_BUS_SUBREAPER_CREDENTIAL_TEST_ARTIFACTS`. Run only in an authorized isolated Linux fixture, never as an operational root CLI or a replacement for the UID65534 suite. Its finite child credential transitions need no setuid executable, installation or account changes. All ten actual original waits/reaps/PID-absence records are retained.

A planned old-acb control records a real GID variant, a discarded wait identity and supervisor124; its one unit/container actually fails1 and is retained as a counterexample. It is not a passing test and does not establish the exact lost historical portal-child identity. Three complete namespace TARs and actual container exits0/0/1 are retained. All profiles are stopped and original configuration/context/old container state are preserved. Private evidence: `delivery/owned-bus-credential-kernel-20261102-41a8a6748f634170/REPORT.md`; qualified independent retained review17dafece accepts the kernel scope.

The fresh S2 full native case failed before its first formal baseline stage; its retained failure remains accepted only for safe preservation/closure. Current kernel results do not pass that case. A separate strict-consumer/source-pin amendment and a new complete seven-stage portal/PDF run remain required. Windows execution remains held.
