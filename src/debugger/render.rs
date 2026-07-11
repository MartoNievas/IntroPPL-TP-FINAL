/*
 
Panel layout for the debugger TUI: a status header, a "current pause point"
panel, a scrollback log of past sample/observe events, and a help footer
with the active keybindings. Pure rendering -- no state mutation happens
here, `app.rs` owns everything this module reads.
 
*/
 
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};