# Maintenance log format

Maintenance logging is machine-local and opt-in at `~/.truss/delivery-log`.
Delivery never creates the directory or file: a missing path is skipped
silently, and deleting the file opts out. Only after a delivery is accepted
and all required checks are green does Control append exactly one physical
line; aborted or incomplete deliveries are not recorded. The line carries an
ISO-8601 UTC timestamp and labelled fields `git-root`, `plan`,
`pull-request` or `none`, `implementation-rounds`, `review-dispositions`,
and `drift-cause`. Tabs separate fields; embedded tabs and newlines become
spaces. Delivery never reads this file for routing, recovery, or runtime
decisions, and its text layout is not a public parsing schema. An append
failure produces a visible warning but does not invalidate or block an
otherwise accepted release.
