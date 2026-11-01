# Private PDF-test bus integration

`pdf_bus_adapter.patch` records the reviewed change to the private native-test adapter. Its original source SHA-256 is `09e0f07c9c39d0022c0bd578ec3b1ad2544ca6a2d36d93a60abeda5819634145`; adapted source is `e61621f81fd8f525bd6e1cf7a6b8e2ab056e82a68afb642671493183e01cc061`. It binds the supervisor, actual bus child and live startup owner ancestry, then requires genuine child/supervisor waits and unchanged strict PID-absence checks.

The adapter belongs to a separately declared private native-test case. This patch is a retained source delta, not an automatic operational plan. Preserve earlier H7 evidence and its failed closure; do not apply it to the old run or reuse its consumed authority. A fresh full case must stage and pin the new helper/adapter, account for their files under the existing limits, and follow the shutdown ordering in `README-bus-subreaper.md`.

Six real Linux UID65534 process-fixture tests passed on 1 November 2026: adopted double-fork exit23 and PID absence; direct exit7; self-SIGTERM with raw15/decoded-15/outer143; honest timeout124 with later real parent reap; failed exec127; and UID rejection before new records. No tests were skipped. All fixture processes were absent before container shutdown. The isolated container and verification VM stopped normally; original H7 container state, Docker context and profile configuration were unchanged. Full bus/portal/native integration remains unverified.

Private runtime evidence is retained under delivery `owned-bus-reaper-smoke-20261101-e1f416d1f5e0e3df`; its complete 57-member TAR is 100,864 bytes, SHA-256 `973caf13758387998b9ad195fafa2d4af6b6b56d660a9a52ab3512029bf6287b`. No product/UI or Windows process was executed.
