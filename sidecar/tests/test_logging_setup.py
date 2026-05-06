import logging
from pathlib import Path

import pytest

from eduport.logging_setup import configure_logging


@pytest.fixture(autouse=True)
def reset_eduport_logger():
    """Clear handlers from the eduport logger before each test to prevent cross-test leakage."""
    logger = logging.getLogger("eduport")
    for h in logger.handlers[:]:
        h.close()
        logger.removeHandler(h)
    yield
    for h in logger.handlers[:]:
        h.close()
        logger.removeHandler(h)


def test_configure_logging_writes_to_given_path(tmp_path: Path):
    log_file = tmp_path / "eduport.log"
    configure_logging(log_file)

    logger = logging.getLogger("eduport.test")
    logger.warning("hello world")

    # Force flush all handlers
    for h in logging.getLogger("eduport").handlers:
        h.flush()

    assert log_file.exists()
    assert "hello world" in log_file.read_text()


def test_configure_logging_is_idempotent(tmp_path: Path):
    log_file = tmp_path / "eduport.log"
    configure_logging(log_file)
    configure_logging(log_file)  # second call must not double-attach handlers
    assert len(logging.getLogger("eduport").handlers) == 1
