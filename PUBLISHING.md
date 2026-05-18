# Publishing & Distribution Plan

Traceability for getting `bush` v0.2.0 published under the name `bush` across registries and package managers. Created 2026-05-16.

## Current state (2026-05-16)

| Field | Value |
|---|---|
| Crate name (Cargo.toml) | `bush` |
| Version | `0.2.0` |
| License | MIT |
| Tests | 274 (181 unit + 93 integration), all green |
| Lint | `cargo clippy --all-targets -- -D warnings` clean |
| Format | `cargo fmt -- --check` clean |
| Repo URL in Cargo.toml | `https://github.com/YOUR_USER/bush` (**TODO: replace with real URL**) |

## Name-availability summary

| Registry / repo | Name `bush` status | Action needed |
|---|---|---|
| **crates.io** | TAKEN by Damian Myrda (moncheeta) — `v0.0.0` placeholder, 2023-03-25, "A shell", no repo, 5 downloads. | Direct contact → transfer (or anti-squatting escalation) |
| **Debian** (`packages.debian.org`) | FREE | Build `.deb`, eventually submit upstream |
| **Ubuntu** (PPAs / Launchpad) | FREE | PPA / custom apt repo |
| **Homebrew** (homebrew-core + taps) | Unknown — TBD before brew submission |
| **AUR** (Arch User Repository) | Unknown — TBD |
| **GitHub** | User-owned namespace — no conflict |

---

## 1. crates.io (`cargo install bush`)

### Decision
**Take over the `bush` name.** Not renaming. Will pursue direct transfer first, anti-squatting policy if owner non-responsive.

### Current policy summary (RFC 3463, late 2023+)
- Transfers are now **owner-mediated**, not team-mediated by default.
- Process: contact owner → owner runs `cargo owner --add USER --crate bush` → owner runs `cargo owner --remove THEMSELVES --crate bush`. Minutes if cooperative.
- crates.io team intervenes **only** when owner is unreachable or refuses without justification, OR when a crate clearly fits the squatting definition: *"exists only to reserve a name … without any genuine functionality, purpose, or significant development activity"*.
- Team retains discretion to delete squatted crates, sometimes without prior notice in egregious cases.

### Owner contact (do not lose this)

| Field | Value |
|---|---|
| Name | Damian Myrda |
| GitHub | https://github.com/moncheeta |
| Email | damian@prime8.dev |
| Website | https://prime8.dev |
| GitLab | https://git.prime8.dev/moncheeta |
| LinkedIn | https://linkedin.com/in/damian-myrda |
| Recent activity | ~22 hrs of code/week on personal portfolio (active) |

### Step 1 — Direct outreach (do this first)

Email to `damian@prime8.dev`:

```
Subject: Quick ask about the `bush` crate name on crates.io

Hi Damian,

I came across your `bush` crate on crates.io (https://crates.io/crates/bush)
and wanted to reach out before doing anything formal. I've built a small Rust
CLI also called `bush` — a `tree` substitute that respects `.gitignore` and
similar files — and I'd love to publish it under that name. The repo is at
[GITHUB_URL], v0.2.0 is ready, ~275 tests passing.

The current `bush` crate looks unused (single `0.0.0` release, no repo linked).
Would you be open to transferring the name? The crates.io process is simple —
you'd run:

    cargo owner --add YOUR_GITHUB_USERNAME --crate bush
    cargo owner --remove moncheeta --crate bush

and that's it. Happy to credit you in the README if that helps.

No worries either way — appreciate you taking a look.

Thanks,
Rehan
```

**Log the date you sent this and the channels tried.** Needed if Step 2 becomes necessary.

| Date | Channel | Response |
|---|---|---|
| _TODO_ | Email (damian@prime8.dev) | |
| _TODO_ | GitHub (issue on a related repo, or direct mention) | |
| _TODO_ | LinkedIn | |

### Step 2 — Anti-squatting escalation (if no response after 2-4 weeks)

Email `help@crates.io`. Subject: *Inactive crate name reclamation: `bush`*.

Evidence to include:
- Single version `0.0.0` published 2023-03-25, never updated (link to https://crates.io/crates/bush/0.0.0)
- Description `"A shell"` — vague, no implementation surfaced anywhere
- `repository` field is empty (no source link)
- 5 total recent downloads (basically no usage)
- ~3 years inactive on the crate itself, while owner remains active on GitHub (so the silence is intentional toward this crate specifically, not loss of access)
- Outreach attempts: list the dates and channels from the table above; include the original email content
- The intended `bush` is ready to ship (link to your repo, version, test count, license)

Argument: the current `bush` matches the RFC 3463 squatting definition. Direct contact has been attempted in good faith. Request review for name reclamation.

### Step 3 — Publish after transfer

Once `cargo owner --list bush` shows you as the owner (or you have a new uploaded version yanking the old):

```bash
# Update Cargo.toml: repository, homepage to real GitHub URL
# Bump version if needed
cargo publish --dry-run                  # verify
cargo publish                            # ship
```

A `~/.cargo/credentials.toml` from `cargo login` is needed once.

### Estimated timeline
- **Best case (owner cooperative):** days
- **Typical:** 2-6 weeks (direct contact + reply lag)
- **Worst (escalation needed):** 2-4 months
- **Catastrophic (rare):** owner objects, escalation denied → fall back to renaming. Mitigated by GitHub Releases path below.

---

## 2. apt (Debian / Ubuntu)

### Status: `bush` name is FREE.
Confirmed via `packages.debian.org` search and local `apt-cache search "^bush$"` — no existing package.

### Paths (recommended order)

**a. Custom apt repo on GitHub Pages** (works immediately, no upstream approval)

Build a `.deb` and host an apt-compatible repo at `https://YOUR_USER.github.io/bush/`. Users:

```bash
curl -fsSL https://YOUR_USER.github.io/bush/pubkey.gpg \
    | sudo gpg --dearmor -o /usr/share/keyrings/bush.gpg
echo "deb [signed-by=/usr/share/keyrings/bush.gpg] https://YOUR_USER.github.io/bush/ stable main" \
    | sudo tee /etc/apt/sources.list.d/bush.list
sudo apt update && sudo apt install bush
```

Build with `cargo-deb`:

```bash
cargo install cargo-deb
cargo deb                  # produces target/debian/bush_0.2.0_amd64.deb
```

Generate the repo metadata with `aptly` or `reprepro`, push to a `gh-pages` branch or a static-site host. CI workflow can build + sign + deploy on every release tag.

**b. Launchpad PPA** (Ubuntu-only, but free hosting)

`ppa:rehan/bush` on Launchpad. Users:

```bash
sudo add-apt-repository ppa:rehan/bush
sudo apt update && sudo apt install bush
```

Pros: zero infrastructure, sign-only-once, auto-builds. Cons: Ubuntu-flavored only (Debian users would prefer the custom repo).

**c. Submit to Debian (long-term)**

Adopt the standard Debian new-package process. Sponsored by a Debian Developer. Quality bar: lintian-clean, manpage, builds from source via debhelper, copyright file. Months to land.

Probably worth it eventually but not the first move.

**d. Submit to Ubuntu (`universe`)**

After Debian inclusion, syncs to Ubuntu via the standard Ubuntu→Debian flow. Or can be MOTU-sponsored separately.

### Recommendation for apt

Start with **(a) custom apt repo on GitHub Pages**. It's complete control, works for both Debian and Ubuntu users, and the `cargo-deb` → static-repo pipeline can be CI-automated. Consider **(c)** later if there's real demand for an `apt install bush` from default Debian.

---

## 3. GitHub Releases + prebuilt binaries (works today, name-independent)

This path doesn't depend on crates.io or apt. Set up regardless.

### Tool: `cargo-dist`

```bash
cargo install cargo-dist
cargo dist init   # interactive; picks Linux/macOS/Windows targets, creates .github/workflows/release.yml
```

On each `git tag vX.Y.Z`, the workflow builds binaries for all configured targets, generates a `shell-install.sh` and `powershell-install.ps1`, attaches everything to a GitHub Release.

Users:

```bash
# any platform — picks the right binary
curl -LsSf https://github.com/YOUR_USER/bush/releases/latest/download/install.sh | sh
```

Also unlocks **cargo-binstall** for free: once metadata points at the GitHub Release artifacts, `cargo binstall bush` downloads the binary instead of compiling.

### When to set this up
**Now.** It's the only distribution path that's name-conflict-free and works today.

---

## 4. Other channels (in priority order, after the above)

| Channel | Effort | Audience | Notes |
|---|---|---|---|
| **Homebrew tap** | Low | Mac users | Your own `homebrew-bush` repo. Users: `brew tap YOUR_USER/bush && brew install bush`. |
| **Homebrew core** | Medium-High | Mac users | PR to `homebrew-core`, maintainer review. Wait until you have some users. |
| **AUR** | Low | Arch users | Publish a `PKGBUILD` to AUR. Mostly write-once. |
| **Nix (nixpkgs)** | Medium | NixOS users | PR to `nixpkgs`. Review-heavy but stable once in. |
| **Snap** | Medium | Universal Linux | `snapcraft.yaml`, Snap Store account. Some friction. |
| **Flatpak** | Medium | Linux desktop | Flathub manifest. Mainly relevant if bush gets a GUI someday — for a CLI, lower payoff. |
| **AppImage** | Low | Universal Linux | Single-file binary. Easy with linuxdeploy. Marginal vs. just downloading the cargo-dist tarball. |
| **Scoop / Chocolatey** | Medium | Windows | Submit manifests. Only if you care about Windows users beyond GitHub Releases. |
| **Docker image** | Low | CI users | `FROM scratch` + static binary. Push to ghcr.io. |

---

## Immediate action list

In priority order:

1. **Today:**
   - Replace `YOUR_USER` in `Cargo.toml` (`repository`, `homepage`) with the real GitHub URL once the repo exists.
   - Send the outreach email to Damian. Log the date.
   - Initialize the GitHub repo, push v0.2.0, tag `v0.2.0`.
2. **This week:**
   - Set up `cargo-dist` for prebuilt binary releases.
   - Set up custom apt repo on GitHub Pages (cargo-deb + aptly).
3. **In 2-4 weeks:**
   - If no reply from Damian, send a follow-up (different channel — GitHub @, LinkedIn).
4. **In 4-6 weeks:**
   - If still no reply, escalate to `help@crates.io` with the evidence in §1 Step 2.
5. **Once `bush` is yours on crates.io:**
   - `cargo publish` v0.2.0.
   - Update README install instructions to include `cargo install bush`.
6. **Later (post-traction):**
   - Submit Homebrew tap → maybe homebrew-core.
   - AUR PKGBUILD.
   - Consider Debian official archive submission.

---

## References

- crates.io policies: https://crates.io/policies
- RFC 3463 — crates.io policy update: https://rust-lang.github.io/rfcs/3463-crates-io-policy-update.html
- Discussion on transfer mechanics: https://github.com/rust-lang/crates.io/discussions/7173
- crates.io API (owner lookup): https://crates.io/api/v1/crates/bush/owners
- Debian package search: https://packages.debian.org/
- cargo-dist: https://github.com/axodotdev/cargo-dist
- cargo-deb: https://github.com/kornelski/cargo-deb
- cargo-binstall: https://github.com/cargo-bins/cargo-binstall
- Launchpad PPA docs: https://help.launchpad.net/Packaging/PPA
