import os
import re
import sys

def validate_changelog(file_path):
    if not os.path.exists(file_path):
        print(f"Error: {file_path} does not exist.")
        return False

    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Required headers in "Keep a Changelog"
    required_headers = [
        r"^# Changelog",
        r"^## \[Unreleased\]",
        r"^## \[0\.1\.0\] - \d{4}-\d{2}-\d{2}",
        r"### Added",
    ]

    missing_headers = []
    for header in required_headers:
        if not re.search(header, content, re.MULTILINE):
            missing_headers.append(header)

    if missing_headers:
        print("Error: Missing or malformed headers:")
        for header in missing_headers:
            print(f"  - {header}")
        return False

    # Check for Semantic Versioning links at the end
    if "[0.1.0]:" not in content:
        print("Error: Missing version link for [0.1.0].")
        return False

    print("Success: CHANGELOG.md is valid.")
    return True

if __name__ == "__main__":
    changelog_path = os.path.join("contract", "CHANGELOG.md")
    if validate_changelog(changelog_path):
        sys.exit(0)
    else:
        sys.exit(1)
