# Pierre MCP Server - Brand Identity & Design System

## Brand Concept: "Holistic Intelligence"

Pierre is a complete fitness intelligence platform that connects AI assistants with fitness data providers. The visual identity represents the human in motion, with data flowing between three core wellness pillars.

> **Design Review**: This design system incorporates recommendations from professional UI/UX analysis to ensure a premium, accessible, and modern fitness app experience.

## Color Palette

### Primary Colors
| Name | Hex | CSS Variable | Usage |
|------|-----|--------------|-------|
| Pierre Violet | `#7C3AED` | `--pierre-violet` | Intelligence, AI, sophistication |
| Pierre Cyan | `#06B6D4` | `--pierre-cyan` | Data flow, connectivity, freshness |

### Three Pillars (Semantic Accents)
| Pillar | Color | Hex | Tailwind Class | Usage |
|--------|-------|-----|----------------|-------|
| Activity | Emerald | `#10B981` | `pierre-activity` | Movement, fitness, energy |
| Nutrition | Amber | `#FBBF24` | `pierre-nutrition` | Food, fuel, nourishment |
| Recovery | Indigo | `#818CF8` | `pierre-recovery` | Rest, sleep, restoration |

> **Dark Mode Optimization**: Amber and Indigo have been adjusted for optimal contrast against dark backgrounds (#0F0F1A). The colors are slightly desaturated with increased brightness to avoid visual "vibration" while maintaining accessibility.

### Neutrals
| Name | Hex | Usage |
|------|-----|-------|
| Deep Space | `#0F0F1A` | Dark backgrounds |
| Slate | `#1E1E2E` | Secondary dark backgrounds |
| Light | `#FAFBFC` | Light backgrounds |

### Gradients
```css
/* Primary gradient (violet to cyan) */
background: linear-gradient(135deg, #7C3AED 0%, #06B6D4 100%);

/* Activity gradient */
background: linear-gradient(135deg, #10B981 0%, #059669 100%);

/* Nutrition gradient */
background: linear-gradient(135deg, #F59E0B 0%, #D97706 100%);

/* Recovery gradient */
background: linear-gradient(135deg, #6366F1 0%, #4F46E5 100%);
```

## Logo

### Primary Logo
- File: `public/pierre-logo.svg`
- Size: 120x120 (scalable)
- Background: Transparent
- Use on light backgrounds

### Icon (Favicon/Small)
- File: `public/pierre-icon.svg`
- Size: 64x64 (scalable)
- Use for favicons, app icons, small displays

### Logo Concept
The logo depicts a stylized running figure composed of interconnected data nodes:
- **Head/Upper body** (Emerald nodes): Activity tracking
- **Core** (Violet-Cyan gradient): Pierre AI intelligence hub
- **Side nodes** (Amber): Nutrition data
- **Lower body** (Indigo nodes): Recovery metrics
- **Connection lines**: Data flow between pillars

### Logo Don'ts
- Don't add backgrounds to the transparent logo
- Don't change the pillar colors
- Don't stretch or distort proportions
- Don't use at sizes smaller than 32px (use icon instead)

## Typography

### Font Stack
```css
/* Primary font - Premium tech aesthetic */
font-family: 'Plus Jakarta Sans', 'Inter', system-ui, -apple-system, sans-serif;

/* Monospace for data/code */
font-family-mono: 'JetBrains Mono', Monaco, Menlo, 'Ubuntu Mono', Consolas, monospace;
```

> **Typography Upgrade**: Plus Jakarta Sans provides a more premium, characterful feel compared to system fonts. It's particularly well-suited for fitness/lifestyle apps. Inter serves as a reliable fallback. For mobile, consider Satoshi as an alternative.

### Font Loading
```html
<!-- Google Fonts -->
<link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700&display=swap" rel="stylesheet">

<!-- Or self-hosted for performance -->
@font-face {
  font-family: 'Plus Jakarta Sans';
  src: url('/fonts/PlusJakartaSans-Variable.woff2') format('woff2');
  font-weight: 400 700;
  font-display: swap;
}
```

### Type Scale
| Name | Size | Usage |
|------|------|-------|
| xs | 0.75rem | Helper text, badges |
| sm | 0.875rem | Body small, labels |
| base | 1rem | Body text |
| lg | 1.125rem | Lead text |
| xl | 1.25rem | Section headers |
| 2xl | 1.5rem | Page headers |
| 3xl | 1.875rem | Hero text |

## Component Patterns

### Cards (Glassmorphism 2.0)

Modern card patterns using backdrop blur and subtle borders for depth:

```css
/* Light theme card */
.card-light {
  background: rgba(255, 255, 255, 0.8);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(0, 0, 0, 0.05);
  border-radius: 1rem;
}

/* Dark theme card - Glassmorphism */
.card-dark {
  background: rgba(30, 30, 46, 0.6);
  backdrop-filter: blur(16px);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 1rem;
}

/* Card with glow effect (for featured items) */
.card-glow {
  background: rgba(30, 30, 46, 0.8);
  backdrop-filter: blur(16px);
  border: 1px solid rgba(124, 58, 237, 0.3);
  box-shadow: 0 0 20px rgba(124, 58, 237, 0.15);
  border-radius: 1rem;
}
```

> **Design Upgrade**: Glassmorphism 2.0 uses backdrop-blur with subtle 1px borders (white at 10% opacity) instead of heavy shadows. This creates depth and hierarchy without visual noise.

### Tailwind Card Classes
```jsx
// Dark theme glassmorphism card
<div className="bg-slate-800/60 backdrop-blur-xl border border-white/10 rounded-2xl p-6">

// Light theme card
<div className="bg-white/80 backdrop-blur-lg border border-black/5 rounded-2xl p-6">

// Featured/glow card
<div className="bg-slate-800/80 backdrop-blur-xl border border-pierre-violet/30 shadow-[0_0_20px_rgba(124,58,237,0.15)] rounded-2xl p-6">
```

### Buttons

**Primary Button** - Gradient with glow effect:
```css
.btn-primary {
  background: linear-gradient(135deg, #7C3AED 0%, #06B6D4 100%);
  color: white;
  border-radius: 0.75rem;
  padding: 0.75rem 1.5rem;
  font-weight: 600;
  transition: all 0.2s ease;
  box-shadow: 0 4px 14px rgba(124, 58, 237, 0.25);
}

.btn-primary:hover {
  box-shadow: 0 6px 20px rgba(124, 58, 237, 0.4);
  transform: translateY(-1px);
}
```

**Pillar Buttons** - Semantic actions with glow:
- Activity: Emerald for fitness-related actions (glow: `rgba(16, 185, 129, 0.3)`)
- Nutrition: Amber for food-related actions (glow: `rgba(251, 191, 36, 0.3)`)
- Recovery: Indigo for rest/sleep-related actions (glow: `rgba(129, 140, 248, 0.3)`)

**Tailwind Button Classes**:
```jsx
// Primary gradient button
<button className="bg-gradient-to-r from-pierre-violet to-pierre-cyan text-white rounded-xl px-6 py-3 font-semibold shadow-lg shadow-pierre-violet/25 hover:shadow-xl hover:shadow-pierre-violet/40 hover:-translate-y-0.5 transition-all">

// Activity button
<button className="bg-pierre-activity text-white rounded-xl px-6 py-3 font-semibold shadow-lg shadow-emerald-500/25 hover:shadow-emerald-500/40 transition-all">
```

### Status Indicators
- Connected/Active: Emerald (`#10B981`)
- Warning/Pending: Amber (`#F59E0B`)
- Error/Disconnected: Red (`#EF4444`)
- Info/Processing: Cyan (`#06B6D4`)

### Three Pillar Badges
When displaying data from different fitness domains:
```jsx
<Badge variant="activity">Running</Badge>   // Emerald
<Badge variant="nutrition">Calories</Badge> // Amber
<Badge variant="recovery">Sleep</Badge>     // Indigo
```

## Micro-Interactions

Subtle animations enhance the premium feel:

```css
/* Hover glow effect */
.hover-glow {
  transition: box-shadow 0.2s ease, transform 0.2s ease;
}
.hover-glow:hover {
  box-shadow: 0 0 20px rgba(124, 58, 237, 0.3);
  transform: translateY(-2px);
}

/* Pulse animation for active states */
@keyframes pulse-glow {
  0%, 100% { box-shadow: 0 0 0 0 rgba(124, 58, 237, 0.4); }
  50% { box-shadow: 0 0 0 8px rgba(124, 58, 237, 0); }
}
.pulse-active {
  animation: pulse-glow 2s infinite;
}

/* Smooth state transitions */
.transition-smooth {
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}
```

### Tailwind Animation Classes
```jsx
// Hover lift effect
className="hover:-translate-y-1 hover:shadow-xl transition-all duration-200"

// Press effect
className="active:scale-95 transition-transform"

// Loading shimmer
className="animate-pulse bg-gradient-to-r from-slate-700 via-slate-600 to-slate-700"
```

## Dashboard Layouts (Bento Grid)

Organize the three pillars using a bento-grid layout:

```jsx
// Bento grid for dashboard
<div className="grid grid-cols-4 gap-4 auto-rows-[140px]">
  {/* Large Activity card - spans 2 cols, 2 rows */}
  <div className="col-span-2 row-span-2 bg-gradient-to-br from-emerald-500/20 to-emerald-600/10 ...">
    Activity
  </div>

  {/* Nutrition card - spans 2 cols */}
  <div className="col-span-2 bg-gradient-to-br from-amber-500/20 to-amber-600/10 ...">
    Nutrition
  </div>

  {/* Recovery card - spans 2 cols */}
  <div className="col-span-2 bg-gradient-to-br from-indigo-500/20 to-indigo-600/10 ...">
    Recovery
  </div>
</div>
```

> **Layout Pattern**: Bento grids allow flexible, magazine-style layouts that work well with the three-pillar system. Each pillar can have variable sizing based on data importance.

## Tailwind Configuration

The brand colors are available in `tailwind.config.js` under the `pierre` namespace:

```js
// tailwind.config.js
module.exports = {
  theme: {
    extend: {
      fontFamily: {
        sans: ['Plus Jakarta Sans', 'Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'Monaco', 'monospace'],
      },
      colors: {
        pierre: {
          violet: '#7C3AED',
          cyan: '#06B6D4',
          activity: '#10B981',
          nutrition: '#FBBF24',  // Optimized for dark mode
          recovery: '#818CF8',   // Optimized for dark mode
          dark: '#0F0F1A',
          slate: '#1E1E2E',
        },
      },
      backdropBlur: {
        xs: '2px',
      },
      boxShadow: {
        'glow-violet': '0 0 20px rgba(124, 58, 237, 0.3)',
        'glow-cyan': '0 0 20px rgba(6, 182, 212, 0.3)',
        'glow-activity': '0 0 20px rgba(16, 185, 129, 0.3)',
        'glow-nutrition': '0 0 20px rgba(251, 191, 36, 0.3)',
        'glow-recovery': '0 0 20px rgba(129, 140, 248, 0.3)',
      },
    },
  },
}
```

### Usage Examples
```jsx
// Primary
className="bg-pierre-violet"
className="bg-pierre-cyan"

// Three Pillars
className="bg-pierre-activity"  // Emerald
className="bg-pierre-nutrition" // Amber (dark-mode optimized)
className="bg-pierre-recovery"  // Indigo (dark-mode optimized)

// Neutrals
className="bg-pierre-dark"
className="bg-pierre-slate"

// Glow shadows
className="shadow-glow-violet"
className="shadow-glow-activity"

// Glassmorphism card
className="bg-pierre-slate/60 backdrop-blur-xl border border-white/10 rounded-2xl"
```

## Accessibility

- Maintain minimum 4.5:1 contrast ratio for text
- Use semantic colors consistently (activity=emerald, etc.)
- Provide text alternatives for color-coded information
- Logo includes proper ARIA labels and descriptions

## Provider Agnosticism

Pierre's branding intentionally avoids referencing specific fitness providers (Strava, Fitbit, etc.). The three-pillar system (Activity, Nutrition, Recovery) is universal and provider-neutral.
