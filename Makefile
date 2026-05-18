.PHONY: prepare publish version-check git-clean git-tag-missing tests test

CRATE_VERSION := $(shell awk -F\" '/^version = / {print $$2; exit}' Cargo.toml)
PUBLISHED_VERSION := $(shell cargo search bush --limit 1 2>/dev/null | awk -F\" '/^bush = / {print $$2; exit}')

prepare: version-check git-clean tests
	@echo "==> prepare OK: version $(CRATE_VERSION) is new, working tree clean, tests green"

version-check:
	@echo "==> Checking version bump"
	@if [ -z "$(CRATE_VERSION)" ]; then \
		echo "ERROR: could not parse version from Cargo.toml"; exit 1; \
	fi
	@echo "    Cargo.toml: $(CRATE_VERSION)"
	@echo "    crates.io:  $(PUBLISHED_VERSION)"
	@if [ "$(CRATE_VERSION)" = "$(PUBLISHED_VERSION)" ]; then \
		echo "ERROR: version $(CRATE_VERSION) already published on crates.io. Bump version in Cargo.toml."; \
		exit 1; \
	fi

git-clean:
	@echo "==> Checking git working tree is clean"
	@if [ -n "$$(git status --porcelain)" ]; then \
		echo "ERROR: uncommitted changes present. Commit or stash before publishing:"; \
		git status --short; \
		exit 1; \
	fi
	@if ! git diff --quiet @{u} HEAD 2>/dev/null; then \
		echo "WARNING: local branch is ahead of upstream; remember to push after publish."; \
	fi

tests:
	@echo "==> Running tests, clippy, fmt"
	cargo fmt -- --check
	cargo clippy --all-targets -- -D warnings
	cargo test --release

publish: prepare
	@echo "==> Publishing $(CRATE_VERSION) to crates.io"
	cargo publish --dry-run
	cargo publish
	@echo "==> Tagging v$(CRATE_VERSION)"
	git tag -a "v$(CRATE_VERSION)" -m "Release v$(CRATE_VERSION)"
	@echo "==> Done. Push the tag with: git push origin v$(CRATE_VERSION)"
