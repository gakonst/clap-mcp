#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_step() {
    echo -e "${GREEN}[STEP]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "clap-mcp" ] || [ ! -d "clap-mcp-derive" ]; then
    print_error "Must be run from the workspace root directory"
    exit 1
fi

# Get the release type (patch, minor, major)
RELEASE_TYPE=${1:-patch}
if [[ ! "$RELEASE_TYPE" =~ ^(patch|minor|major)$ ]]; then
    print_error "Usage: $0 [patch|minor|major]"
    exit 1
fi

print_step "Starting $RELEASE_TYPE release process..."

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    print_error "There are uncommitted changes. Please commit or stash them first."
    exit 1
fi

# Make sure we're on main/master branch
CURRENT_BRANCH=$(git branch --show-current)
if [[ "$CURRENT_BRANCH" != "main" ]] && [[ "$CURRENT_BRANCH" != "master" ]]; then
    print_warning "Not on main/master branch. Current branch: $CURRENT_BRANCH"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Pull latest changes
print_step "Pulling latest changes..."
git pull origin "$CURRENT_BRANCH"

# Run tests
print_step "Running tests..."
cargo test --workspace

# Run clippy
print_step "Running clippy..."
cargo clippy --workspace --all-features --all-targets -- -D warnings

# Check formatting
print_step "Checking formatting..."
cargo +nightly fmt --all --check

# Build the project
print_step "Building the project..."
cargo build --workspace --release

# Get current version
CURRENT_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
print_step "Current version: $CURRENT_VERSION"

# Calculate new version
IFS='.' read -ra VERSION_PARTS <<< "$CURRENT_VERSION"
MAJOR=${VERSION_PARTS[0]}
MINOR=${VERSION_PARTS[1]}
PATCH=${VERSION_PARTS[2]}

case $RELEASE_TYPE in
    patch)
        PATCH=$((PATCH + 1))
        ;;
    minor)
        MINOR=$((MINOR + 1))
        PATCH=0
        ;;
    major)
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        ;;
esac

NEW_VERSION="$MAJOR.$MINOR.$PATCH"
print_step "New version will be: $NEW_VERSION"

# Update version in workspace Cargo.toml
print_step "Updating version in Cargo.toml files..."
sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
rm Cargo.toml.bak

# Update dependency version in clap-mcp/Cargo.toml
sed -i.bak "s/clap-mcp-derive = { path = \"..\/clap-mcp-derive\", version = \".*\" }/clap-mcp-derive = { path = \"..\/clap-mcp-derive\", version = \"$NEW_VERSION\" }/" clap-mcp/Cargo.toml
rm clap-mcp/Cargo.toml.bak

# Update lock file
print_step "Updating Cargo.lock..."
cargo update --workspace

# Run tests again with new version
print_step "Running tests with new version..."
cargo test --workspace

# Generate changelog for this release
print_step "Generating changelog..."
CHANGELOG=$(git log --pretty=format:"- %s" "v$CURRENT_VERSION"..HEAD 2>/dev/null || echo "- Initial release")

# Create git tag
TAG="v$NEW_VERSION"
print_step "Creating git tag: $TAG"

# Commit version bump
git add -A
git commit -m "chore: release $NEW_VERSION

- Bump version from $CURRENT_VERSION to $NEW_VERSION
- Update dependency versions
- Update Cargo.lock"

# Create annotated tag
git tag -a "$TAG" -m "Release $NEW_VERSION"

# Show what would be published
print_step "Checking what will be published..."
echo "clap-mcp-derive:"
cargo publish --dry-run --package clap-mcp-derive
echo
echo "clap-mcp:"
cargo publish --dry-run --package clap-mcp

# Confirm before proceeding
echo
print_warning "Ready to publish version $NEW_VERSION and push to git"
read -p "Continue with publishing? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_warning "Aborting. You can manually publish later with:"
    echo "  cargo publish --package clap-mcp-derive"
    echo "  cargo publish --package clap-mcp"
    echo "  git push origin $CURRENT_BRANCH"
    echo "  git push origin $TAG"
    exit 0
fi

# Publish to crates.io (derive macro first, then main crate)
print_step "Publishing clap-mcp-derive to crates.io..."
cargo publish --package clap-mcp-derive

# Wait a bit for crates.io to index the derive crate
print_step "Waiting for crates.io to index clap-mcp-derive..."
sleep 30

print_step "Publishing clap-mcp to crates.io..."
cargo publish --package clap-mcp

# Push to git
print_step "Pushing to git..."
git push origin "$CURRENT_BRANCH"
git push origin "$TAG"

# Create GitHub release
print_step "Creating GitHub release..."

# Create release notes
RELEASE_NOTES="## What's Changed

$CHANGELOG

## Installation

\`\`\`toml
[dependencies]
clap-mcp = \"$NEW_VERSION\"
\`\`\`

## Example

\`\`\`rust
use clap::{Parser, Subcommand};
use clap_mcp::McpMode;

#[derive(Parser, McpMode)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    #[arg(long)]
    #[mcp(mode_flag)]
    mcp: bool,
}
\`\`\`

**Full Changelog**: https://github.com/gakonst/clap-mcp/compare/v$CURRENT_VERSION...$TAG"

# Create the release using gh
gh release create "$TAG" \
    --title "Release $NEW_VERSION" \
    --notes "$RELEASE_NOTES" \
    --latest

print_step "Release $NEW_VERSION completed successfully!"
echo
echo "✅ Version bumped to $NEW_VERSION"
echo "✅ Published to crates.io"
echo "✅ Git tag $TAG created and pushed"
echo "✅ GitHub release created"
echo
echo "View the release at: https://github.com/gakonst/clap-mcp/releases/tag/$TAG"