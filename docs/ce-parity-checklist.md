# CE Parity Checklist

This checklist defines the minimum CE-like behaviors Squalr should support and how to verify them.

## Scanning
- Exact value scan (4 bytes)
  - Expected: results appear and spinner stops
  - Verify: attach to Notepad, search for a known value
- Changed / Unchanged
  - Expected: next scan filters results properly
- Increased / Decreased
  - Expected: next scan filters results properly
- Unknown initial value
  - Expected: initial scan works without a value
- Array of Bytes (AOB)
  - Expected: pattern finds matching memory
- String (UTF-8)
  - Expected: text search returns matching addresses

## Results Table
- Selection + copy
  - Expected: Ctrl+A selects all, Ctrl+C copies Address/Value/Prev/Type
- Context menu actions
  - Expected: Change Value, Disassemble Here, View Memory Region, Pointer Scan for this Address
- Change Value workflow
  - Expected: only selected rows are written; unreadable values do not crash

## Disassembler
- Reopen after close
  - Expected: Windows menu restores the tab
- Go to address
  - Expected: module+offset and absolute addresses supported
- Unreadable bytes
  - Expected: displayed as ?? without crashing

## Memory Viewer
- View Memory Region from results
  - Expected: region is highlighted and scrolled into view
- Unreadable regions
  - Expected: bytes ??, ASCII .

## Pointer Scanner
- Start / progress / cancel
  - Expected: task runs and reports progress; cancel works
- Results display
  - Expected: base + offsets shown; copy path works
- Prefill from results
  - Expected: “Pointer scan for this address” fills target
