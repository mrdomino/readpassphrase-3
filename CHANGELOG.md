# Changelog
## [unreleased]

### 🚀 Features

- [**breaking**] Use libbsd-sys for ffi where possible (#22)

## [1.0.2] - 2025-10-01

### 📚 Documentation

- Link to crate documentation in cargo.toml
- Owned example reuses buffer on utf8 error
- Linkify reference in MAX_CAPACITY

### 🎨 Styling

- Sort Cargo.toml entries
- Make ffi module always private
- Change changelog format

### 🧪 Testing

- Test that zeroize sets string length to 0

### ⚙️ Miscellaneous Tasks

- Add link to changelog

## [1.0.1] - 2025-09-13

### 📚 Documentation

- Change keywords
- Change getpass wording to match man-page

### 🎨 Styling

- Use map_err instead of match on result
- Consistently spell NUL in comments

### ⚙️ Miscellaneous Tasks

- Tag v1.0.1

## [1.0.0] - 2025-09-12
- First stable release

[unreleased]: https://github.com/mrdomino/readpassphrase-3/compare/v1.0.2..HEAD
[1.0.2]: https://github.com/mrdomino/readpassphrase-3/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/mrdomino/readpassphrase-3/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/mrdomino/readpassphrase-3/tree/v1.0.0
