# Administration state matrix

| Module | Unsupported / stale | Guarded preview | Unknown / verification |
|---|---|---|---|
| Processes | missing adapter; PID/start changed | exact signal + start identity + user | inspect same identity; exited vs replaced vs unavailable |
| Packages | distro unsupported; candidate changed | exact names/versions/options; partial risk | inspect each installed package; partial/mismatch/unavailable |
| Files | root denied; symlink/revision changed | canonical path + bytes + diff + expected revision | bounded read/hash; mismatch/unavailable; no blind write retry |
| Storage | missing field shows unavailable + time | no mutation in read-first fixture | read-only observation; no fabricated zero or generic rollback |
| Network | missing rule adapter; rule revision changed | unavailable here; future exact rule/interface + connectivity risk | loss after submit is unknown; inspect independently before new intent |
| Jobs | unsupported cron/timer; revision changed | exact schedule + zone + execution user + privilege | inspect registered entry/next run; not proof job executed |
| Access | protected account; changed key/grant revision | exact group/key metadata + privilege/lockout impact | inspect grant/key metadata; no promise about active sessions |

Every module also has shared loading, empty, validation/error, unauthorized, disconnected and stale notices in the preview. Guarded modules use shared approval-required/expired, running, refusal, failed-before-submit and unknown-after-submit views. Read-only modules never gain mutation buttons through those states. Touch/keyboard controls use shared components; tables scroll internally and detailed annotations stay readable as one column.
