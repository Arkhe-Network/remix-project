import pytest
import asyncio
from sidecar.client import CircuitBreaker

@pytest.mark.asyncio
async def test_circuit_breaker_state_transitions():
    cb = CircuitBreaker(max_failures=2, recovery_timeout=0.1)

    assert cb.is_available() is True

    cb.record_failure()
    assert cb.is_available() is True

    cb.record_failure()
    assert cb.is_available() is False

    await asyncio.sleep(0.2)
    assert cb.check() is True

    cb.record_success(0.1)
    assert cb.is_available() is True
