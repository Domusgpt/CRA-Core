# Hello World Atlas

This is an example Atlas demonstrating CRA concepts.

## Purpose

This Atlas provides simple greeting functionality to demonstrate:

- Context block injection
- Action resolution
- TRACE event emission
- Policy enforcement

## Capabilities

### greeting.send

Send a greeting message.

**Constraints:**
- Message must be non-empty
- Message length < 1000 characters

### greeting.customize

Customize greeting templates.

**Constraints:**
- Requires elevated scopes
- Must follow template format

## Usage

Request resolution with goal: "Send a greeting"

The resolver will return:
- Context about greeting formats
- Allowed greeting.send action
- Deny rules for inappropriate content
