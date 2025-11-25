# Pierre MCP Server - Brand Identity & Design System

## Brand Concept: "Holistic Intelligence"

Pierre is a complete fitness intelligence platform that connects AI assistants with fitness data providers. The visual identity represents the human in motion, with data flowing between three core wellness pillars.

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
| Nutrition | Amber | `#F59E0B` | `pierre-nutrition` | Food, fuel, nourishment |
| Recovery | Indigo | `#6366F1` | `pierre-recovery` | Rest, sleep, restoration |

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
font-family: system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
font-family-mono: Monaco, Menlo, 'Ubuntu Mono', Consolas, monospace;
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

### Cards
- Background: White (`#FFFFFF`) on light theme
- Border: `pierre-gray-200`
- Border radius: `0.5rem` (md)
- Shadow: Subtle on hover

### Buttons
- Primary: Pierre Blue gradient or solid
- Activity: Emerald for fitness-related actions
- Nutrition: Amber for food-related actions
- Recovery: Indigo for rest/sleep-related actions

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

## Tailwind Configuration

The brand colors are available in `tailwind.config.js` under the `pierre` namespace:

```js
// Primary
className="bg-pierre-violet"
className="bg-pierre-cyan"

// Three Pillars
className="bg-pierre-activity"  // Emerald
className="bg-pierre-nutrition" // Amber
className="bg-pierre-recovery"  // Indigo

// Neutrals
className="bg-pierre-dark"
className="bg-pierre-slate"
```

## Accessibility

- Maintain minimum 4.5:1 contrast ratio for text
- Use semantic colors consistently (activity=emerald, etc.)
- Provide text alternatives for color-coded information
- Logo includes proper ARIA labels and descriptions

## Provider Agnosticism

Pierre's branding intentionally avoids referencing specific fitness providers (Strava, Fitbit, etc.). The three-pillar system (Activity, Nutrition, Recovery) is universal and provider-neutral.
