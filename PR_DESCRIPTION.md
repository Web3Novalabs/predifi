# Health Check Enhancements

This PR implements several health check enhancements to improve system reliability and observability.

## Changes

- Added Redis cache health checking to both `/health` and `/api/v1/health` endpoints
- Added price cache health checking to both `/health` and `/api/v1/health` endpoints
- Added configurable RPC health check timeout settings
- Added retry logic with exponential backoff for RPC health checks
- Enhanced error reporting with detailed error information in health responses

## Related Issues

- Fixes #1003: Add Redis cache health check
- Fixes #984: Implement enhanced error handling and retry logic
- Fixes #986: Implement configurable timeout settings
- Fixes #991: Add price cache health check

## Testing

- Added comprehensive tests for all new health check functionality
- Verified that health endpoints return appropriate status codes (200 for healthy, 503 for degraded)
- Verified that dependency status is correctly reported in the response body
- Verified that error details are included when dependencies are unreachable

## Configuration

New environment variables added:

- `RPC_HEALTH_TIMEOUT_SECS`: Configurable timeout for RPC health checks (default: 2)
- `RPC_HEALTH_RETRY_COUNT`: Configurable retry count for RPC health checks (default: 3)

## Impact

These changes improve the reliability and observability of the health check system, making it easier to diagnose issues with dependencies like Redis cache, price cache, and Stellar RPC.