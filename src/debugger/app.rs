/*
 
Debugger application state: holds the currently-paused effect, the event
log shown in the history panel, and the snapshot stack used for read-only
"back" navigation. Owns the RNG used to resolve `sample` sites in step
mode (single draw) and continue mode (auto-run to the next
breakpoint/Done).
 
*/
