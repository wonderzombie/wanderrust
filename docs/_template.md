---
title: Short Descriptive Title
status: draft        # draft | proposed | building | done | superseded | abandoned
created: YYYY-MM-DD
updated: YYYY-MM-DD
tags: [ecs, rendering, refactor]
---

<!--
HOW TO USE THIS TEMPLATE

Copy to docs/<snake_case_name>.md. Delete sections you don't need — an
empty section is worse than a missing one.

Only three sections are load-bearing: Summary, Objective, Design.
Everything else earns its place or gets cut.

The point of the template isn't to make you write neatly on the first
pass. Write however you write. The template is where the mess *lands*
afterward — Alternatives Considered is the container for the dead ends,
Open Questions is the container for the thought-loops. Nothing gets
deleted, it just gets sorted.
-->

# TITLE

## SUMMARY

Two to four sentences. What changes, in one breath. A reader should be
able to decide in ten seconds whether to keep reading.

Good test: if you handed someone only this section, could they describe
the change to a third person?

## OBJECTIVE

What's wrong today, and what "done" looks like.

**Not in scope:**

- The things you are deliberately not doing
- Naming these up front prevents scope creep and prevents readers from
  asking "but what about X"

## BACKGROUND

Optional. Only what a reader needs in order to follow the Design: the
existing types, the current shape of the code, relevant engine
semantics.

If a reader who knows the codebase would skip this, cut it.

## DESIGN

The proposal. Types, systems, message flow. Code sketches welcome —
they don't need to compile.

Lead with the decision, then justify it. Not the other way around.

## ALTERNATIVES CONSIDERED

Where the dead ends live. This is not a graveyard — it's the reasoning
that makes the Design credible. Keep them.

### Alternative: <name>

What it was, and one line on why not.

## OPEN QUESTIONS

- Things you don't know yet
- Things you're deferring
- Perfectly fine to ship a doc with these unanswered

## WORK ITEMS

- [ ] Concrete, checkable
- [ ] Ordered if order matters

## OUTCOME

Filled in *after* the work is done. What actually happened, what the
design got wrong, what you'd tell yourself before starting.

This is the section that makes a design doc worth keeping around. A doc
without it decays into a description of a plan reality ignored.
