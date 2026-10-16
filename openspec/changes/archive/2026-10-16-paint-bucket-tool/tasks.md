# Tasks: paint-bucket-tool

## 1. Engine

- [x] 1.1 `pictura_paint::bucket` (`flood`, `fill`, pattern fill); unit tests.

## 2. Bridge

- [x] 2.1 `bucket_fill_at` (layer or composite flood, anti-alias, selection); one "Paint Bucket" state.

## 3. Tool

- [x] 3.1 Click handler in `tool_fills.cpp`; bar in `options_bar_fill.cpp`; catalog row enabled.

## 4. Verification

- [x] 4.1 `paintBucketTool` in the Qt Test suite `tst_fill_tools` (first a self-test, code 553, now retired); `bash scripts/verify-fast.sh`.
