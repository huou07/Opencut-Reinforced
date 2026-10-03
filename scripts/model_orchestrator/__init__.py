"""Deterministic control layer for model orchestration (Layer A).

Owns task state, locks, packets, guards, attempts, and recovery.
Works with zero external models available; models are workers, never truth.
"""
