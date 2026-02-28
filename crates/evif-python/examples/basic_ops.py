"""Basic EVIF operations example."""

import asyncio
from evif import EvifClient
from evif.file_handle import FileHandle


async def basic_operations():
    """Demonstrate basic file operations."""
    async with EvifClient("http://localhost:8080") as client:
        # 1. List files in directory
        print("=== Listing files ===")
        files = await client.ls("/local/tmp")
        for file in files:
            print(f"  {file.name}: {file.size} bytes")

        # 2. Write a file
        print("\n=== Writing file ===")
        content = "Hello, EVIF!\nThis is a test file."
        bytes_written = await client.write("/local/tmp/test.txt", content)
        print(f"  Written {bytes_written} bytes")

        # 3. Read the file back
        print("\n=== Reading file ===")
        data = await client.cat("/local/tmp/test.txt")
        print(f"  Content: {data.decode()}")

        # 4. Get file metadata
        print("\n=== File metadata ===")
        info = await client.stat("/local/tmp/test.txt")
        print(f"  Size: {info.size} bytes")
        print(f"  Mode: {oct(info.mode)}")
        print(f"  Is file: {info.is_file}")
        print(f"  Is dir: {info.is_dir}")

        # 5. Copy file
        print("\n=== Copying file ===")
        await client.cp("/local/tmp/test.txt", "/local/tmp/test_copy.txt")
        print("  File copied")

        # 6. List again to see new file
        print("\n=== Updated listing ===")
        files = await client.ls("/local/tmp")
        for file in files:
            print(f"  {file.name}: {file.size} bytes")

        # 7. Clean up
        print("\n=== Cleaning up ===")
        await client.rm("/local/tmp/test.txt")
        await client.rm("/local/tmp/test_copy.txt")
        print("  Files removed")


async def file_handle_example():
    """Demonstrate file handle usage."""
    async with EvifClient("http://localhost:8080") as client:
        print("\n=== File Handle Example ===")

        # Open handle
        handle_info = await client.open_handle(
            "/local/tmp/handle_test.txt",
            flags=FileHandle.READ_WRITE | FileHandle.CREATE,
            lease=60,
        )

        # Use context manager for auto-cleanup
        async with FileHandle(
            handle_info.id,
            "/local/tmp/handle_test.txt",
            client,
        ) as handle:
            # Write data
            await handle.write(b"Line 1\n")
            await handle.write(b"Line 2\n")
            await handle.write(b"Line 3\n")

            # Seek to start
            await handle.seek(0)

            # Read line by line
            print("  Reading lines:")
            while True:
                data = await handle.read(100)
                if not data:
                    break
                print(f"    {data.decode()}", end="")

            # Clean up
            await client.rm("/local/tmp/handle_test.txt")
            print("  Handle test completed")


async def mount_example():
    """Demonstrate plugin mounting."""
    async with EvifClient("http://localhost:8080") as client:
        print("\n=== Mount Example ===")

        # List current mounts
        mounts = await client.mounts()
        print("  Current mounts:")
        for mount in mounts:
            print(f"    {mount.path} -> {mount.plugin}")

        # Mount local filesystem
        print("\n  Mounting local filesystem...")
        await client.mount(
            "localfs",
            "/local",
            {"root": "/tmp"},
        )
        print("  Mounted at /local")

        # List files
        files = await client.ls("/local")
        print(f"  Files in /local: {len(files)} items")


async def main():
    """Run all examples."""
    print("EVIF Python SDK Examples\n")

    try:
        await basic_operations()
        await file_handle_example()
        await mount_example()

        print("\n=== All examples completed ===")
    except Exception as e:
        print(f"\nError: {e}")


if __name__ == "__main__":
    asyncio.run(main())
