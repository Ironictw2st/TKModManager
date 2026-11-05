# "Send report": posting a mod report to the developers' Discord. The report itself stays in
# English because the developers read it.

report-sent = Report sent.
report-sent-with-dump = Report and crash dump sent.
report-unreachable = cannot reach Discord: { $error }
# $status is an HTTP status such as "413 Payload Too Large"; $message is Discord's answer
report-refused = Discord refused the report ({ $status }): { $message }
# The dump was too big for one message and went out as several in the same thread
report-sent-with-parts = Report and crash dump sent (the dump in { $parts } parts).
report-sent-dump-too-big = Report sent. The crash dump was too big to upload; the report has a summary of the crash.
# $part is the number of the part that failed; $error is why
report-parts-failed = Report sent, but the crash dump did not fully upload (part { $part }): { $error }
