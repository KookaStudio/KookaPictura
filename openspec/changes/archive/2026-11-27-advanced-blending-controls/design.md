# Design

## Context

The existing layer fields already have compositor and PSD support, and Layer Style edits use a live transaction with accept and cancel.

## Goals / Non-Goals

Use the existing transaction for all three flags. Blend If belongs to #264.

## Decisions

Extend the existing blending value keys and checkbox bindings, rather than creating a separate state or transaction. Existing copy/paste and persistence retain the same fields.

## Risks / Trade-offs

A binding mismatch could silently edit the wrong flag; test each independently and across the dialog lifecycle. The adjacent Blend If work shares the page; #695 lands first and #264 rebases.
