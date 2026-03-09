"""Configuration for EVIF Memory Client."""

from typing import Optional
from pydantic import BaseModel, Field


class MemoryConfig(BaseModel):
    """Configuration for the EVIF Memory client.

    Args:
        api_url: Base URL of the EVIF Memory API
        api_key: API key for authentication
        timeout: Request timeout in seconds
        max_retries: Maximum number of retries for failed requests
    """

    api_url: str = Field(
        default="http://localhost:8080",
        description="Base URL of the EVIF Memory API"
    )
    api_key: Optional[str] = Field(
        default=None,
        description="API key for authentication"
    )
    timeout: int = Field(
        default=30,
        description="Request timeout in seconds"
    )
    max_retries: int = Field(
        default=3,
        description="Maximum number of retries"
    )

    class Config:
        frozen = True
