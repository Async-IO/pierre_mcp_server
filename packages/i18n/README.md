# @pierre/i18n

Unified internationalization package for Pierre web and mobile applications.

## Features

- 🌍 Support for English, Spanish, and French
- ⚛️ Works with both React (web) and React Native (mobile)
- 🔄 Automatic language persistence
- 📝 Type-safe translation keys
- 🎯 Easy to extend with new languages
- 🔌 Based on industry-standard i18next

## Installation

This package is part of the Pierre monorepo workspace and is automatically linked when you run:

```bash
bun install
```

## Usage

### Web (frontend/)

1. **Initialize i18n in your app entry point:**

```tsx
// App.tsx or main.tsx
import { initI18n } from '@pierre/i18n';

// Initialize before rendering
initI18n();

function App() {
  return <YourApp />;
}
```

2. **Use translations in components:**

```tsx
import { useTranslation } from '@pierre/i18n';

function LoginPage() {
  const { t } = useTranslation();

  return (
    <div>
      <h1>{t('common.welcome')}</h1>
      <input placeholder={t('common.email')} />
      <input type="password" placeholder={t('common.password')} />
      <button>{t('common.login')}</button>
    </div>
  );
}
```

3. **Add language switcher:**

```tsx
import { useLanguageSwitcher, SUPPORTED_LANGUAGES, LANGUAGE_NAMES } from '@pierre/i18n';

function LanguageSwitcher() {
  const { currentLanguage, changeLanguage } = useLanguageSwitcher();

  return (
    <select value={currentLanguage} onChange={(e) => changeLanguage(e.target.value)}>
      {SUPPORTED_LANGUAGES.map((lang) => (
        <option key={lang} value={lang}>
          {LANGUAGE_NAMES[lang]}
        </option>
      ))}
    </select>
  );
}
```

### Mobile (frontend-mobile/)

1. **Initialize i18n in your app entry point:**

```tsx
// App.tsx
import { initI18n } from '@pierre/i18n';

// Initialize before rendering
initI18n();

export default function App() {
  return <YourApp />;
}
```

2. **Use translations in screens/components:**

```tsx
import { useTranslation } from '@pierre/i18n';
import { View, Text, TextInput, Button } from 'react-native';

function LoginScreen() {
  const { t } = useTranslation();

  return (
    <View>
      <Text>{t('common.welcome')}</Text>
      <TextInput placeholder={t('common.email')} />
      <TextInput placeholder={t('common.password')} secureTextEntry />
      <Button title={t('common.login')} />
    </View>
  );
}
```

3. **Add language switcher (use native version for mobile):**

```tsx
import { useLanguageSwitcherNative, SUPPORTED_LANGUAGES, LANGUAGE_NAMES } from '@pierre/i18n';
import { View, Text, TouchableOpacity } from 'react-native';

function LanguageSwitcher() {
  const { currentLanguage, changeLanguage } = useLanguageSwitcherNative();

  return (
    <View>
      {SUPPORTED_LANGUAGES.map((lang) => (
        <TouchableOpacity
          key={lang}
          onPress={() => changeLanguage(lang)}
          style={{ opacity: currentLanguage === lang ? 1 : 0.6 }}
        >
          <Text>{LANGUAGE_NAMES[lang]}</Text>
        </TouchableOpacity>
      ))}
    </View>
  );
}
```

## Translation Structure

Translations are organized into namespaces for better organization:

- `common` - Common UI elements (buttons, labels, actions)
- `auth` - Authentication and registration
- `chat` - Chat and messaging
- `coaches` - Coach management
- `settings` - Settings and preferences
- `social` - Social features
- `insights` - Analytics and insights
- `providers` - Provider connections
- `errors` - Error messages
- `validation` - Form validation messages

## Supported Languages

- 🇺🇸 English (`en`) - Default
- 🇪🇸 Spanish (`es`)
- 🇫🇷 French (`fr`)

## Adding New Languages

1. Create a new directory in `src/locales/` (e.g., `de` for German)
2. Copy `en/translation.json` to your new directory
3. Translate all strings
4. Update `SUPPORTED_LANGUAGES` in `src/config.ts`
5. Add the language name to `LANGUAGE_NAMES`
6. Import and add the translations to `defaultI18nConfig.resources`

## Translation Key Format

Use dot notation to access nested keys:

```tsx
t('common.welcome')        // "Welcome"
t('auth.loginFailed')      // "Login failed"
t('chat.typeMessage')      // "Type a message..."
```

## Interpolation

For dynamic values, use interpolation:

```tsx
// In translation file:
{
  "validation": {
    "minLength": "Minimum length is {{min}} characters"
  }
}

// In code:
t('validation.minLength', { min: 8 })
// Output: "Minimum length is 8 characters"
```

## Type Safety

The package provides type-safe translation keys through the `TranslationKeys` interface. TypeScript will provide autocomplete for available keys.

## Best Practices

1. **Use descriptive keys**: Prefer `auth.invalidCredentials` over `error1`
2. **Keep translations short**: Mobile screens have limited space
3. **Test all languages**: Verify UI layout with longer translations (German, French)
4. **Use placeholders**: For dynamic content, use interpolation
5. **Maintain consistency**: Use the same terms across languages
6. **Context matters**: Consider cultural differences in translations

## Troubleshooting

### Translations not updating

Make sure you've initialized i18n before rendering:

```tsx
initI18n();
```

### Language not persisting

- **Web**: Check browser localStorage
- **Mobile**: Ensure AsyncStorage permissions are granted

### Missing translations

Check the browser/Metro console for missing key warnings. i18next will fall back to the English translation if a key is missing.

## License

MIT OR Apache-2.0
