import logging
from logging.handlers import RotatingFileHandler
from pathlib import Path


def configure_logging(log_file: Path, level: int = logging.INFO) -> None:
    """Attach a rotating file handler to the eduport logger.

    Idempotent: calling more than once with the same path does not duplicate handlers.
    The logger level is only set on the first call; subsequent calls with a different
    level are silently ignored to keep behaviour predictable.
    """
    log_file = log_file.resolve()
    log_file.parent.mkdir(parents=True, exist_ok=True)
    logger = logging.getLogger("eduport")

    if any(
        isinstance(h, RotatingFileHandler) and Path(h.baseFilename) == log_file
        for h in logger.handlers
    ):
        return

    logger.setLevel(level)
    handler = RotatingFileHandler(
        log_file, maxBytes=10 * 1024 * 1024, backupCount=3, encoding="utf-8"
    )
    handler.setFormatter(
        logging.Formatter("%(asctime)s %(levelname)s %(name)s %(message)s")
    )
    logger.addHandler(handler)
