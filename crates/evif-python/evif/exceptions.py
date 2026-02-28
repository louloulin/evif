"""EVIF Client exceptions."""


class EvifError(Exception):
    """Base exception for all EVIF errors."""

    def __init__(self, message: str, code: int = 500):
        self.message = message
        self.code = code
        super().__init__(self.message)


class ClientError(EvifError):
    """Exception raised for client-side errors."""

    pass


class AuthenticationError(EvifError):
    """Exception raised for authentication failures."""

    def __init__(self, message: str = "Authentication failed"):
        super().__init__(message, 401)


class FileNotFoundError(EvifError):
    """Exception raised when a file or directory is not found."""

    def __init__(self, path: str):
        super().__init__(f"File not found: {path}", 404)
        self.path = path


class PermissionError(EvifError):
    """Exception raised for permission denied errors."""

    def __init__(self, message: str = "Permission denied"):
        super().__init__(message, 403)


class TimeoutError(EvifError):
    """Exception raised when a request times out."""

    def __init__(self, message: str = "Request timed out"):
        super().__init__(message, 408)


class ValidationError(EvifError):
    """Exception raised for input validation errors."""

    def __init__(self, message: str):
        super().__init__(message, 400)


class ServerError(EvifError):
    """Exception raised for server-side errors."""

    def __init__(self, message: str = "Internal server error"):
        super().__init__(message, 500)
