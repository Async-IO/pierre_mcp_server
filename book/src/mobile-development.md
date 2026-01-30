<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Mobile Development Guide

Complete guide for setting up and developing the Pierre Mobile app.

## Prerequisites

### Required Software

| Software | Version | Purpose |
|----------|---------|---------|
| Node.js | 20+ | JavaScript runtime |
| Bun | 1.0+ | Package manager |
| Xcode | 15+ | iOS development (macOS only) |
| Android Studio | Hedgehog+ | Android development |
| Watchman | Latest | File watching (recommended) |

### Platform-Specific Setup

#### macOS (iOS Development)

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install CocoaPods
sudo gem install cocoapods

# Install Watchman (recommended)
brew install watchman
```

#### All Platforms (Android Development)

1. Install [Android Studio](https://developer.android.com/studio)
2. Install Android SDK via SDK Manager:
   - Android SDK Platform 34
   - Android SDK Build-Tools 34.x
   - Android Emulator
   - Android SDK Platform-Tools

3. Set environment variables:
```bash
# Add to ~/.bashrc, ~/.zshrc, or equivalent
export ANDROID_HOME=$HOME/Android/Sdk
export PATH=$PATH:$ANDROID_HOME/emulator
export PATH=$PATH:$ANDROID_HOME/platform-tools
```

## Installation

### 1. Clone Repository

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
```

### 2. Install Mobile Dependencies

```bash
cd frontend-mobile
bun install
```

### 3. Start Backend Server

The mobile app requires the Pierre backend. In a separate terminal:

```bash
# From repository root
./bin/setup-and-start.sh
```

Or manually:
```bash
source .envrc
cargo run --release
```

Server runs on `http://localhost:8081`.

### 4. Start Development Server

```bash
cd frontend-mobile
bun start
```

This starts the Expo development server with options to run on different platforms.

## Running the App

### iOS Simulator (macOS only)

```bash
bun run ios
```

Or press `i` in the Expo CLI.

### Android Emulator

```bash
bun run android
```

Or press `a` in the Expo CLI.

### Physical Device

1. Install Expo Go app on your device
2. Scan the QR code shown in terminal
3. Configure API endpoint for your network:

```bash
# Set your machine's IP address
export EXPO_PUBLIC_API_URL=http://192.168.1.100:8081
bun start
```

## Development Workflow

### Project Structure

```
frontend-mobile/
├── App.tsx               # Entry point
├── src/
│   ├── components/ui/    # Reusable components (Button, Card, Input)
│   ├── constants/        # Theme configuration
│   ├── contexts/         # React contexts (Auth, WebSocket)
│   ├── navigation/       # Navigation setup
│   ├── screens/          # App screens
│   ├── services/         # API client
│   └── types/            # TypeScript definitions
├── __tests__/            # Unit tests
└── e2e/                  # E2E tests (Detox)
```

### Key Files

| File | Purpose |
|------|---------|
| `src/services/api.ts` | API client with auth handling |
| `src/contexts/AuthContext.tsx` | Authentication state management |
| `src/contexts/WebSocketContext.tsx` | Real-time chat streaming |
| `src/constants/theme.ts` | Colors, spacing, typography |
| `src/navigation/RootNavigator.tsx` | Navigation structure |

### Available Scripts

```bash
bun start           # Start Expo development server
bun run ios         # Run on iOS Simulator
bun run android     # Run on Android Emulator
bun run typecheck   # TypeScript type checking
bun run lint        # ESLint
bun test            # Run unit tests
bun run test:coverage  # Tests with coverage report
bun run e2e:build   # Build for Detox E2E
bun run e2e:test    # Run Detox E2E tests
```

## Testing

### Unit Tests

Unit tests use Jest with React Native Testing Library:

```bash
# Run all tests
bun test

# Run with coverage
bun run test:coverage

# Run specific test file
bun test -- Button.test.tsx

# Watch mode
bun run test:watch
```

Test files are in `__tests__/` directory:
- `Button.test.tsx` - Button component tests
- `Card.test.tsx` - Card component tests
- `Input.test.tsx` - Input component tests
- `AuthContext.test.tsx` - Authentication tests
- `WebSocketContext.test.tsx` - WebSocket tests
- `api.test.ts` - API service tests
- `theme.test.ts` - Theme constants tests
- `types.test.ts` - Type definition tests

### E2E Tests (Detox)

E2E tests require iOS Simulator:

```bash
# Build app for testing
bun run e2e:build

# Run E2E tests
bun run e2e:test
```

> **Note**: E2E tests may be flaky on CI due to simulator timing issues.

## Debugging

### React Native Debugger

1. Shake device or press `m` in Expo CLI
2. Select "Debug Remote JS"
3. Use Chrome DevTools or React Native Debugger app

### Network Debugging

View network requests in Expo CLI or use Flipper:

```bash
# Install Flipper
brew install flipper
```

### Common Issues

#### Metro bundler cache

```bash
npx expo start --clear
```

#### iOS pod issues

```bash
cd ios
pod install --repo-update
cd ..
```

#### Android Gradle issues

```bash
cd android
./gradlew clean
cd ..
```

#### Connection refused on device

1. Ensure backend is running
2. Use machine IP, not localhost
3. Check firewall allows port 8081
4. Verify both devices on same network

## Building for Production

### iOS

```bash
# Prebuild native project
npx expo prebuild --platform ios

# Build with Xcode
open ios/pierremobile.xcworkspace
# Select "Any iOS Device" and build
```

### Android

```bash
# Prebuild native project
npx expo prebuild --platform android

# Build APK
cd android
./gradlew assembleRelease
```

### EAS Build (Recommended)

For production builds, use Expo Application Services:

```bash
# Install EAS CLI
bun add -g eas-cli

# Configure
eas build:configure

# Build for iOS
eas build --platform ios

# Build for Android
eas build --platform android
```

## CI/CD

Mobile tests run in GitHub Actions on every push:

- **Unit Tests**: Run on Ubuntu, fast (~1 minute)
- **E2E Tests**: Run on macOS with iOS Simulator (optional, flaky)

See `.github/workflows/mobile-tests.yml` for configuration.

## Related Documentation

- [Frontend Mobile README](../frontend-mobile/README.md) - App overview and components
- [Getting Started](getting-started.md) - Backend setup
- [Authentication](authentication.md) - OAuth and JWT details
- [Contributing](contributing.md) - Contribution guidelines
