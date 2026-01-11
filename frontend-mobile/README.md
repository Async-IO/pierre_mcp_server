# Pierre Mobile

React Native mobile app for Pierre Fitness Platform, providing a conversational AI interface for fitness coaching and data analysis.

## Features

- **AI Chat Interface**: Conversational UI with markdown rendering and real-time streaming
- **Fitness Provider Integration**: Connect to Strava, Garmin, Fitbit, WHOOP, COROS via OAuth
- **Activity Tracking**: View and analyze your fitness activities
- **Training Insights**: Get AI-powered training recommendations and analysis
- **Offline Support**: Secure token storage with AsyncStorage and SecureStore

## Tech Stack

- **Framework**: React Native with Expo SDK 54
- **Navigation**: React Navigation (Drawer + Native Stack)
- **Styling**: NativeWind (TailwindCSS for React Native)
- **State Management**: React Query + Context API
- **Testing**: Jest + React Native Testing Library + Detox (E2E)
- **TypeScript**: Full type safety throughout

## Project Structure

```
frontend-mobile/
├── src/
│   ├── components/       # Reusable UI components
│   │   └── ui/           # Button, Card, Input
│   ├── constants/        # Theme colors, spacing, typography
│   ├── contexts/         # Auth and WebSocket providers
│   ├── navigation/       # App navigation structure
│   ├── screens/          # App screens
│   │   ├── auth/         # Login, Register, PendingApproval
│   │   ├── chat/         # Main chat interface
│   │   ├── connections/  # Provider OAuth connections
│   │   └── settings/     # User settings
│   ├── services/         # API client and utilities
│   └── types/            # TypeScript type definitions
├── __tests__/            # Unit tests
├── e2e/                  # Detox E2E tests
├── App.tsx               # App entry point
└── package.json
```

## Quick Start

### Prerequisites

- Node.js 20+
- npm or yarn
- Xcode (for iOS development)
- Android Studio (for Android development)
- Pierre backend server running on `localhost:8081`

### Installation

```bash
# Navigate to mobile directory
cd frontend-mobile

# Install dependencies
npm install --legacy-peer-deps

# Start Expo development server
npm start
```

### Running on Devices

```bash
# iOS Simulator
npm run ios

# Android Emulator
npm run android

# Web (experimental)
npm run web
```

## Development

### Type Checking

```bash
npm run typecheck
```

### Linting

```bash
npm run lint
```

### Testing

```bash
# Run unit tests
npm test

# Run with coverage
npm run test:coverage

# Watch mode
npm run test:watch
```

### E2E Testing (Detox)

```bash
# Build for iOS simulator
npm run e2e:build

# Run E2E tests
npm run e2e:test
```

## Configuration

### API Endpoint

The app connects to the Pierre backend. Configure the endpoint:

```typescript
// src/services/api.ts
const API_BASE_URL = process.env.EXPO_PUBLIC_API_URL || 'http://localhost:8081';
```

For physical devices, replace `localhost` with your machine's IP address.

### Environment Variables

Create a `.env` file or use Expo's environment configuration:

```bash
EXPO_PUBLIC_API_URL=http://192.168.1.100:8081
```

## Architecture

### Authentication Flow

1. User registers or logs in via OAuth 2.0
2. JWT token stored securely in AsyncStorage
3. CSRF token included for state-changing requests
4. Automatic token refresh on expiration
5. Auth failure triggers logout and navigation to login

### WebSocket Streaming

Real-time chat responses use WebSocket connections:
- Connect when entering a conversation
- Receive streamed response chunks
- Accumulate and display with markdown rendering
- Handle connection errors gracefully

### Navigation Structure

```
RootNavigator
├── AuthStack (unauthenticated)
│   ├── LoginScreen
│   ├── RegisterScreen
│   └── PendingApprovalScreen
└── AppDrawer (authenticated)
    ├── ChatScreen (main)
    ├── ConnectionsScreen
    └── SettingsScreen
```

## UI Components

### Button

```tsx
import { Button } from '@/components/ui';

<Button
  title="Submit"
  onPress={handleSubmit}
  variant="primary"  // primary | secondary | ghost | danger
  size="md"          // sm | md | lg
  loading={isLoading}
  disabled={!isValid}
/>
```

### Card

```tsx
import { Card } from '@/components/ui';

<Card variant="elevated" noPadding={false}>
  <Text>Card content</Text>
</Card>
```

### Input

```tsx
import { Input } from '@/components/ui';

<Input
  label="Email"
  placeholder="Enter email"
  error={errors.email}
  keyboardType="email-address"
  secureTextEntry
  showPasswordToggle
/>
```

## Testing

The app includes 92 unit tests covering:

- **UI Components**: Button, Card, Input rendering and interactions
- **AuthContext**: Login, logout, registration, state management
- **WebSocketContext**: Connection lifecycle, message streaming
- **API Service**: Request formatting and response handling
- **Types**: Type definition validation

Run tests with:
```bash
npm test
```

## Troubleshooting

### Metro bundler issues

```bash
# Clear cache and restart
npx expo start --clear
```

### iOS build issues

```bash
# Clean and reinstall pods
cd ios && pod install --repo-update && cd ..
```

### Android build issues

```bash
# Clean Gradle cache
cd android && ./gradlew clean && cd ..
```

### Connection to backend fails

1. Ensure Pierre server is running on port 8081
2. For physical devices, use your machine's IP instead of localhost
3. Check firewall allows connections on port 8081

## Contributing

See [Contributing Guide](../docs/contributing.md) and [Mobile Development Guide](../docs/mobile-development.md).

## License

See root [LICENSE](../LICENSE) file.
