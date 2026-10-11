---
name: pdf-extractor
description: Extract text and tables from PDF documents. Use when the user asks to read, parse or pull data out of a PDF file.
---
# PDF Extractor

Pull text and tables out of PDF files page by page.

## Workflow

1. Open the PDF and count its pages.
2. Extract text per page. See [edge cases](references/notes.md) for scanned files.

```bash
pdftotext -layout input.pdf -
```
