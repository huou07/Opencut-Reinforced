"""Model orchestrator V2 control-plane package (M0: contracts and adoption schema).

This package currently contains only the frozen contract/authority validation
surface. It has no runtime dispatch, worker, store, promotion, or supervisor
entrypoint; those are later implementation phases.
"""

from __future__ import annotations

__all__ = ["contracts"]
