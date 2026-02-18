# Issue 348: Market Template and Cloning System

## Description

Simplify market creation by allowing users to create "Templates" (e.g., standard Soccer Match template). New markets can then be cloned from these templates, reducing storage costs and parameter errors.

## Tasks

- Implement a `MarketTemplate` storage structure.
- Add `create_pool_from_template` function.
- Enforce immutable template parameters (e.g., resolving logic).

## Dependencies

- Issue #307
- Issue #308
- Issue #305
