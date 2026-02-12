# Login Component i18n Integration Example

## Before (Current Implementation)

```tsx
// frontend/src/components/Login.tsx (excerpt)
export default function Login({ onNavigateToRegister }: LoginProps) {
  const [error, setError] = useState('');
  
  const loginAction = useAsyncAction({
    action: () => login(email, password),
    onError: (err: unknown) => {
      const apiError = err as { response?: { data?: { error?: string } } };
      const errorMsg = apiError.response?.data?.error || 'Login failed';
      if (errorMsg === 'invalid_grant' || errorMsg.includes('Invalid') || errorMsg.includes('credentials')) {
        setError('Invalid email or password');
      } else {
        setError(errorMsg);
      }
    },
  });

  const handleGoogleSignIn = async () => {
    try {
      await signInWithGoogle();
    } catch (err: unknown) {
      const firebaseError = err as { code?: string; message?: string };
      if (firebaseError.code === 'auth/network-request-failed') {
        setError('Network error. Please check your connection.');
      } else {
        setError(firebaseError.message || 'Google sign-in failed');
      }
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-pierre-gray-900 via-black to-pierre-gray-900 flex items-center justify-center p-4">
      <div className="w-full max-w-md">
        <div className="text-center mb-8">
          <PierreLogo />
          <h1 className="text-3xl font-bold text-white mt-4 mb-2">Pierre</h1>
          <p className="text-pierre-gray-400">Your AI Fitness Intelligence Platform</p>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          <Input
            type="email"
            placeholder="Email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
          <Input
            type={showPassword ? 'text' : 'password'}
            placeholder="Password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          
          <Button type="submit" loading={loginAction.loading}>
            Sign In
          </Button>

          {isFirebaseEnabled() && (
            <Button type="button" onClick={handleGoogleSignIn} loading={isGoogleLoading}>
              <svg className="w-5 h-5 mr-2" viewBox="0 0 24 24">
                {/* Google icon SVG */}
              </svg>
              Sign in with Google
            </Button>
          )}

          {error && (
            <div className="mt-4 p-3 bg-red-500/10 border border-red-500/50 rounded-lg">
              <p className="text-red-400 text-sm">{error}</p>
            </div>
          )}

          {onNavigateToRegister && (
            <p className="text-center text-pierre-gray-400 text-sm">
              Don't have an account?{' '}
              <button onClick={onNavigateToRegister} className="text-pierre-violet hover:underline">
                Sign up
              </button>
            </p>
          )}
        </form>
      </div>
    </div>
  );
}
```

## After (With i18n Integration)

```tsx
// frontend/src/components/Login.tsx (with i18n)
import { useTranslation } from '@pierre/i18n';

export default function Login({ onNavigateToRegister }: LoginProps) {
  const { t } = useTranslation(); // Add translation hook
  const [error, setError] = useState('');
  
  const loginAction = useAsyncAction({
    action: () => login(email, password),
    onError: (err: unknown) => {
      const apiError = err as { response?: { data?: { error?: string } } };
      const errorMsg = apiError.response?.data?.error || t('auth.loginFailed');
      if (errorMsg === 'invalid_grant' || errorMsg.includes('Invalid') || errorMsg.includes('credentials')) {
        setError(t('auth.invalidCredentials')); // Translated
      } else {
        setError(errorMsg);
      }
    },
  });

  const handleGoogleSignIn = async () => {
    try {
      await signInWithGoogle();
    } catch (err: unknown) {
      const firebaseError = err as { code?: string; message?: string };
      if (firebaseError.code === 'auth/network-request-failed') {
        setError(t('errors.network')); // Translated
      } else {
        setError(firebaseError.message || t('auth.loginFailed')); // Translated
      }
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-pierre-gray-900 via-black to-pierre-gray-900 flex items-center justify-center p-4">
      <div className="w-full max-w-md">
        <div className="text-center mb-8">
          <PierreLogo />
          <h1 className="text-3xl font-bold text-white mt-4 mb-2">{t('common.appName')}</h1>
          <p className="text-pierre-gray-400">Your AI Fitness Intelligence Platform</p>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          <Input
            type="email"
            placeholder={t('common.email')} // Translated
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
          <Input
            type={showPassword ? 'text' : 'password'}
            placeholder={t('common.password')} // Translated
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          
          <Button type="submit" loading={loginAction.loading}>
            {t('common.login')} {/* Translated */}
          </Button>

          {isFirebaseEnabled() && (
            <Button type="button" onClick={handleGoogleSignIn} loading={isGoogleLoading}>
              <svg className="w-5 h-5 mr-2" viewBox="0 0 24 24">
                {/* Google icon SVG */}
              </svg>
              {t('auth.signInWithGoogle')} {/* Translated */}
            </Button>
          )}

          {error && (
            <div className="mt-4 p-3 bg-red-500/10 border border-red-500/50 rounded-lg">
              <p className="text-red-400 text-sm">{error}</p>
            </div>
          )}

          {onNavigateToRegister && (
            <p className="text-center text-pierre-gray-400 text-sm">
              {t('auth.alreadyHaveAccount')}{' '} {/* Translated */}
              <button onClick={onNavigateToRegister} className="text-pierre-violet hover:underline">
                {t('common.register')} {/* Translated */}
              </button>
            </p>
          )}
        </form>
      </div>
    </div>
  );
}
```

## Key Changes

1. **Import translation hook**: `import { useTranslation } from '@pierre/i18n';`
2. **Initialize hook**: `const { t } = useTranslation();`
3. **Replace hardcoded strings** with `t()` calls:
   - `"Email"` → `t('common.email')`
   - `"Password"` → `t('common.password')`
   - `"Sign In"` → `t('common.login')`
   - `"Sign in with Google"` → `t('auth.signInWithGoogle')`
   - Error messages → Translation keys

## Translation Keys Used

From `packages/i18n/src/locales/*/translation.json`:

```json
{
  "common": {
    "appName": "Pierre Fitness Intelligence",
    "email": "Email" / "Correo electrónico" / "E-mail",
    "password": "Password" / "Contraseña" / "Mot de passe",
    "login": "Sign In" / "Iniciar sesión" / "Se connecter",
    "register": "Sign Up" / "Registrarse" / "S'inscrire"
  },
  "auth": {
    "signInWithGoogle": "Sign in with Google" / "Iniciar sesión con Google" / "Se connecter avec Google",
    "invalidCredentials": "Invalid email or password" / "Correo o contraseña inválidos" / "E-mail ou mot de passe invalide",
    "loginFailed": "Login failed" / "Error al iniciar sesión" / "Échec de la connexion",
    "alreadyHaveAccount": "Don't have an account?" / "¿Ya tienes una cuenta?" / "Vous avez déjà un compte ?"
  },
  "errors": {
    "network": "Network error. Please check your connection." / "Error de red..." / "Erreur réseau..."
  }
}
```

## Testing the Integration

### 1. Manual Testing

```bash
# Start the dev server
cd frontend
bun run dev

# Open browser, navigate to login page
# Change language in settings or browser console:
window.i18n.changeLanguage('es')  # Spanish
window.i18n.changeLanguage('fr')  # French
window.i18n.changeLanguage('en')  # Back to English
```

### 2. Visual Verification

**English:**
- Input placeholder: "Email"
- Button text: "Sign In"
- Link text: "Don't have an account? Sign up"

**Spanish:**
- Input placeholder: "Correo electrónico"
- Button text: "Iniciar sesión"
- Link text: "¿Ya tienes una cuenta? Registrarse"

**French:**
- Input placeholder: "E-mail"
- Button text: "Se connecter"
- Link text: "Vous avez déjà un compte ? S'inscrire"

### 3. Layout Testing

Verify that:
- [ ] Buttons don't overflow with longer text
- [ ] Input placeholders fit within fields
- [ ] Error messages display correctly
- [ ] No text is cut off or truncated

## Mobile Integration (Similar Pattern)

```tsx
// frontend-mobile/src/screens/auth/LoginScreen.tsx
import { useTranslation } from '@pierre/i18n';

export function LoginScreen() {
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

## Summary

This example demonstrates how straightforward it is to add internationalization to existing components:

1. **One import**: `import { useTranslation } from '@pierre/i18n';`
2. **One hook call**: `const { t } = useTranslation();`
3. **Replace strings**: Change `"Text"` to `t('key')`

The same pattern works identically for both web and mobile, making it easy to maintain consistency across platforms.
