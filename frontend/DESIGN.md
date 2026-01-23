# Design System: Pierre UI
**Project ID:** 13228861249011906351 (Stitch Reference)

> This document serves as the source of truth for prompting Stitch to generate new screens that align with Pierre's design language. Based on analysis of the Pierre UI Redesign Stitch project.

## 1. Visual Theme & Atmosphere

**Mood:** Dark, premium, sophisticated with cosmic undertones
**Aesthetic Philosophy:** "Holistic Intelligence" - representing the human in motion with data flowing between wellness pillars
**Density:** Spacious and breathable with generous padding
**Character:** Modern glassmorphism with subtle depth, accented by violet glow effects

The interface evokes a sense of advanced technology while remaining approachable and wellness-focused. Think "premium fitness tech meets AI intelligence."

## 2. Color Palette & Roles

### Primary Brand Colors
| Descriptive Name | Hex Code | Functional Role |
|-----------------|----------|-----------------|
| Pierre Violet | `#7C3AED` | Primary actions, AI indicators, brand identity |
| Pierre Cyan | `#06B6D4` | Data flow, connectivity, secondary accents |

### Three Pillars (Semantic Accents)
| Pillar | Descriptive Name | Hex Code | Functional Role |
|--------|-----------------|----------|-----------------|
| Activity | Energetic Emerald | `#10B981` | Movement, fitness, running, cycling |
| Nutrition | Warm Amber | `#FBBF24` | Food tracking, calories, meals |
| Recovery | Calming Indigo | `#818CF8` | Sleep, rest, restoration metrics |

### Background Hierarchy
| Descriptive Name | Hex Code | Functional Role |
|-----------------|----------|-----------------|
| Deep Space | `#0F0F1A` | Primary dark background, app canvas |
| Elevated Slate | `#1E1E2E` | Card backgrounds, elevated surfaces |
| Cosmic Gray | `#2A2A3E` | Hover states, tertiary surfaces |

### Glass & Transparency
| Descriptive Name | Value | Functional Role |
|-----------------|-------|-----------------|
| Glass Base | `rgba(30, 30, 46, 0.6)` | Standard card background |
| Glass Dense | `rgba(30, 30, 46, 0.8)` | Featured/prominent cards |
| Glass Light | `rgba(255, 255, 255, 0.05)` | AI message bubbles |
| Glass Border | `rgba(255, 255, 255, 0.1)` | Card borders, dividers |
| Glass Border Strong | `rgba(255, 255, 255, 0.15)` | Emphasized borders |

## 3. Typography Rules

### Font Families
| Platform | Primary Font | Fallback Stack |
|----------|-------------|----------------|
| Web | Plus Jakarta Sans | Inter, system-ui, sans-serif |
| Mobile | Lexend | System, sans-serif |
| Monospace | JetBrains Mono | Monaco, Menlo, monospace |

> **Lexend** is specifically chosen for mobile due to its optimized readability on small screens.

### Weight Usage
| Context | Weight | Usage |
|---------|--------|-------|
| Headings | 600-700 (Semibold/Bold) | Page titles, section headers |
| Subheadings | 500 (Medium) | Card titles, labels |
| Body | 400 (Regular) | Paragraph text, descriptions |
| Emphasis | 500-600 | Important values, metrics |

### Type Scale
| Name | Size | Usage |
|------|------|-------|
| Hero | 1.875rem (30px) | Dashboard headlines |
| Title | 1.5rem (24px) | Page headers |
| Subtitle | 1.25rem (20px) | Section headers |
| Body Large | 1.125rem (18px) | Lead paragraphs |
| Body | 1rem (16px) | Standard text |
| Small | 0.875rem (14px) | Labels, captions |
| Tiny | 0.75rem (12px) | Badges, timestamps |

## 4. Component Stylings

### Buttons

**Primary CTA (Pill-shaped with glow):**
```css
background: linear-gradient(135deg, #7C3AED 0%, #06B6D4 100%);
border-radius: 9999px; /* Pill shape */
box-shadow: 0 4px 14px rgba(124, 58, 237, 0.25);
/* Hover: intensified glow */
box-shadow: 0 6px 20px rgba(124, 58, 237, 0.4);
```

**Pillar Buttons:**
- Activity: Emerald fill with `shadow-glow-activity`
- Nutrition: Amber fill with `shadow-glow-nutrition`
- Recovery: Indigo fill with `shadow-glow-recovery`

### Cards/Containers (Glassmorphism 2.0)

**Standard Card:**
```css
background: rgba(30, 30, 46, 0.6);
backdrop-filter: blur(16px);
border: 1px solid rgba(255, 255, 255, 0.1);
border-radius: 1rem; /* 16px */
```

**Featured Card (with glow):**
```css
background: rgba(30, 30, 46, 0.8);
backdrop-filter: blur(16px);
border: 1px solid rgba(124, 58, 237, 0.3);
box-shadow: 0 0 20px rgba(124, 58, 237, 0.15);
border-radius: 1.5rem; /* 24px */
```

**Prominent Card (large radius):**
```css
border-radius: 2rem; /* 32px */
/* For hero sections, major CTAs */
```

### Inputs/Forms

**Glass Input:**
```css
background: rgba(255, 255, 255, 0.03);
backdrop-filter: blur(20px);
border: 1px solid rgba(255, 255, 255, 0.08);
border-radius: 0.75rem; /* 12px */
/* Focus state */
border-color: rgba(124, 58, 237, 0.5);
box-shadow: 0 0 0 3px rgba(124, 58, 237, 0.1);
```

### Chat Bubbles (AI Coach)

**AI Message:**
```css
background: rgba(255, 255, 255, 0.05);
backdrop-filter: blur(12px);
border: 1px solid rgba(255, 255, 255, 0.1);
border-radius: 1rem;
```

**User Message:**
```css
background: rgba(124, 58, 237, 0.9);
backdrop-filter: blur(8px);
border: 1px solid rgba(255, 255, 255, 0.1);
border-radius: 1rem;
```

## 5. Layout Principles

### Spacing Scale
| Name | Value | Usage |
|------|-------|-------|
| xs | 0.25rem (4px) | Tight inline spacing |
| sm | 0.5rem (8px) | Icon gaps, compact elements |
| md | 1rem (16px) | Standard component padding |
| lg | 1.5rem (24px) | Card padding, section gaps |
| xl | 2rem (32px) | Major section separations |
| 2xl | 3rem (48px) | Hero spacing |

### Grid & Alignment
- **Bento Grid**: Use for dashboards with varied card sizes
- **Content Width**: Max 1280px for main content
- **Sidebar Width**: 280px (collapsed: 72px)
- **Card Gap**: 1rem (16px) standard, 1.5rem (24px) for dashboard

### Responsive Breakpoints
| Name | Width | Target |
|------|-------|--------|
| Mobile | < 640px | Phone portrait |
| Tablet | 640-1024px | Tablet, phone landscape |
| Desktop | > 1024px | Desktop, laptop |

## 6. Motion & Interactions

### Micro-interactions
```css
/* Hover lift effect */
transition: transform 0.2s ease, box-shadow 0.2s ease;
transform: translateY(-2px);

/* Glow intensification */
box-shadow: 0 0 20px rgba(124, 58, 237, 0.3);

/* Press effect */
transform: scale(0.98);
```

### Animation Timing
| Type | Duration | Easing |
|------|----------|--------|
| Hover | 200ms | ease-out |
| Press | 100ms | ease-in |
| Modal | 300ms | cubic-bezier(0.4, 0, 0.2, 1) |
| Page | 400ms | ease-in-out |

### Pulse Glow (Active States)
```css
@keyframes pulse-glow {
  0%, 100% { box-shadow: 0 0 0 0 rgba(124, 58, 237, 0.4); }
  50% { box-shadow: 0 0 0 8px rgba(124, 58, 237, 0); }
}
```

## 7. Accessibility Notes

- Maintain minimum 4.5:1 contrast ratio for text
- Pillar colors are WCAG AA compliant on dark backgrounds
- All interactive elements have visible focus states
- Animations respect `prefers-reduced-motion`

## 8. Stitch Prompting Tips

When generating new screens, reference this document by including:
- "Use Pierre's Deep Space (#0F0F1A) background"
- "Apply glassmorphism cards with 16px blur and white/10 borders"
- "Buttons should be pill-shaped with violet gradient glow"
- "Use Lexend font for mobile screens"
- "Include the three-pillar color system for Activity/Nutrition/Recovery"
