# Authentication & Authorization

This document outlines the policies and flows for authentication, authorization, and security permissions within the project.

## Authentication Flows
- Details on OAuth2, JWT, and credential passing.
- Implementation of the Edge Proxy Pattern (CredentialInjectionProxy).

## Authorization
- Role-Based Access Control (RBAC) and Attribute-Based Access Control (ABAC).
- Security policies preventing AI tokens and keys from being directly handled by the models.

## Secret Management
- Usage of Burn-After-Use (BAU) policies.
- `secrecy` and `zeroize` usage in memory allocations.
