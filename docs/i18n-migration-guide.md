# i18n Migration Guide

This guide shows how to migrate existing components to use the unified i18n system.

## Setup Steps

### 1. Add i18n to Frontend Dependencies

The `@pierre/i18n` package is already linked via workspace. No additional dependencies needed.

### 2. Initialize i18n

#### Web (frontend/src/main.tsx)

```tsx
import { createRoot } from 'react-dom/client';
import { initI18n } from '@pierre/i18n';
import App from './App';

// Initialize i18n before rendering
initI18n();

createRoot(document.getElementById('root')!).render(<App />);
```

#### Mobile (frontend-mobile/App.tsx)

```tsx
import { initI18n } from '@pierre/i18n';
import { Navigation } from './src/navigation';

// Initialize i18n before rendering
initI18n();

export default function App() {
  return <Navigation />;
}
```

## Migration Examples

### Example 1: Login Component (Web)

**Before:**
```tsx
export default function Login() {
  return (
    <div>
      <h1>Welcome</h1>
      <Input placeholder="Email" />
      <Input type="password" placeholder="Password" />
      <Button>Sign In</Button>
      <button>Sign in with Google</button>
    </div>
  );
}
```

**After:**
```tsx
import { useTranslation } from '@pierre/i18n';

export default function Login() {
  const { t } = useTranslation();
  
  return (
    <div>
      <h1>{t('common.welcome')}</h1>
      <Input placeholder={t('common.email')} />
      <Input type="password" placeholder={t('common.password')} />
      <Button>{t('common.login')}</Button>
      <button>{t('auth.signInWithGoogle')}</button>
    </div>
  );
}
```

### Example 2: Settings with Language Switcher

**Web:**
```tsx
import { LanguageSwitcher } from './LanguageSwitcher';
import { useTranslation } from '@pierre/i18n';

function UserSettings() {
  const { t } = useTranslation();
  
  return (
    <div>
      <h2>{t('common.settings')}</h2>
      <div>
        <label>{t('common.language')}</label>
        <LanguageSwitcher />
      </div>
    </div>
  );
}
```

**Mobile:**
```tsx
import { LanguageSwitcher } from '../components/LanguageSwitcher';
import { useTranslation } from '@pierre/i18n';

function SettingsScreen() {
  const { t } = useTranslation();
  
  return (
    <View>
      <Text>{t('common.settings')}</Text>
      <LanguageSwitcher />
    </View>
  );
}
```

## Priority Components to Migrate

1. Login/Register screens (high user visibility)
2. Main navigation and tabs
3. Settings pages
4. Error messages and toasts
5. Chat interface
6. Coach management
7. Social features

For full documentation, see `/packages/i18n/README.md`
