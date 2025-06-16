# Pierre MCP Server - Design System

## Brand Identity

### Brand Positioning
**Pierre MCP Server** - The foundation for enterprise fitness data access.
- **Mission**: Provide reliable, secure, and scalable API access to fitness data
- **Vision**: Become the go-to platform for AI assistants and developers integrating fitness data
- **Values**: Reliability, Performance, Security, Developer Experience

### Brand Personality
- **Professional**: Enterprise-grade quality and polish
- **Trustworthy**: Security-first approach with transparent operations  
- **Innovative**: Cutting-edge MCP protocol and AI integration
- **Developer-Friendly**: Intuitive APIs and comprehensive documentation

---

## Visual Identity

### Logo Concept
```
🗿 PIERRE
   MCP Server
```
- **Primary Symbol**: 🗿 (Moai/Stone face) - represents stability and foundation
- **Typography**: Modern, clean sans-serif
- **Tagline**: "The Foundation of Fitness Data"

### Color Palette

#### Primary Colors
```css
--pierre-blue-50:  #eff6ff;   /* Light backgrounds */
--pierre-blue-100: #dbeafe;   /* Subtle highlights */
--pierre-blue-500: #3b82f6;   /* Primary brand color */
--pierre-blue-600: #2563eb;   /* Interactive elements */
--pierre-blue-700: #1d4ed8;   /* Active states */
--pierre-blue-900: #1e3a8a;   /* Headers, text */
```

#### Secondary Colors  
```css
--pierre-gray-50:  #f9fafb;   /* Page backgrounds */
--pierre-gray-100: #f3f4f6;   /* Card backgrounds */
--pierre-gray-300: #d1d5db;   /* Borders */
--pierre-gray-500: #6b7280;   /* Secondary text */
--pierre-gray-700: #374151;   /* Primary text */
--pierre-gray-900: #111827;   /* Headers */
```

#### Status Colors
```css
--pierre-green-50:  #f0fdf4;  /* Success backgrounds */
--pierre-green-500: #22c55e;  /* Success indicators */
--pierre-green-600: #16a34a;  /* Success buttons */

--pierre-yellow-50:  #fefce8; /* Warning backgrounds */
--pierre-yellow-500: #eab308; /* Warning indicators */
--pierre-yellow-600: #ca8a04; /* Warning buttons */

--pierre-red-50:  #fef2f2;    /* Error backgrounds */
--pierre-red-500: #ef4444;    /* Error indicators */
--pierre-red-600: #dc2626;    /* Error buttons */

--pierre-purple-50:  #faf5ff; /* Premium backgrounds */
--pierre-purple-500: #a855f7; /* Premium indicators */
--pierre-purple-600: #9333ea; /* Premium buttons */
```

#### API Tier Colors
```css
--tier-trial:        #eab308; /* Yellow - Trial */
--tier-starter:      #3b82f6; /* Blue - Starter */
--tier-professional: #22c55e; /* Green - Professional */
--tier-enterprise:   #a855f7; /* Purple - Enterprise */
```

### Typography

#### Font Stack
```css
/* Primary: Modern, clean sans-serif */
--font-primary: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;

/* Code/Monospace: Developer-friendly */
--font-mono: 'JetBrains Mono', 'Fira Code', 'SF Mono', monospace;

/* Display: For headlines and logos */
--font-display: 'Inter', system-ui, sans-serif;
```

#### Font Scales
```css
/* Headers */
--text-4xl: 2.25rem;   /* 36px - Main headers */
--text-3xl: 1.875rem;  /* 30px - Section headers */
--text-2xl: 1.5rem;    /* 24px - Card headers */
--text-xl:  1.25rem;   /* 20px - Subsections */
--text-lg:  1.125rem;  /* 18px - Large text */

/* Body */
--text-base: 1rem;     /* 16px - Body text */
--text-sm:   0.875rem; /* 14px - Secondary text */
--text-xs:   0.75rem;  /* 12px - Labels, captions */
```

#### Font Weights
```css
--font-light:     300;
--font-normal:    400;
--font-medium:    500;
--font-semibold:  600;
--font-bold:      700;
```

---

## Component Library

### Layout Components

#### Container
```css
.pierre-container {
  max-width: 1280px;
  margin: 0 auto;
  padding: 0 1rem;
}

.pierre-container-fluid {
  width: 100%;
  padding: 0 1rem;
}
```

#### Grid System
```css
.pierre-grid {
  display: grid;
  gap: 1.5rem;
}

.pierre-grid-cols-1 { grid-template-columns: repeat(1, 1fr); }
.pierre-grid-cols-2 { grid-template-columns: repeat(2, 1fr); }
.pierre-grid-cols-3 { grid-template-columns: repeat(3, 1fr); }
.pierre-grid-cols-4 { grid-template-columns: repeat(4, 1fr); }

/* Responsive variants */
@media (min-width: 768px) {
  .pierre-md-grid-cols-2 { grid-template-columns: repeat(2, 1fr); }
  .pierre-md-grid-cols-3 { grid-template-columns: repeat(3, 1fr); }
  .pierre-md-grid-cols-4 { grid-template-columns: repeat(4, 1fr); }
}
```

### Card Components

#### Base Card
```css
.pierre-card {
  background: white;
  border-radius: 0.75rem;
  border: 1px solid var(--pierre-gray-200);
  padding: 1.5rem;
  box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1);
  transition: box-shadow 0.2s ease;
}

.pierre-card:hover {
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
}

.pierre-card-header {
  border-bottom: 1px solid var(--pierre-gray-200);
  padding-bottom: 1rem;
  margin-bottom: 1.5rem;
}

.pierre-card-title {
  font-size: var(--text-lg);
  font-weight: var(--font-semibold);
  color: var(--pierre-gray-900);
  margin: 0;
}
```

#### Stat Card
```css
.pierre-stat-card {
  background: linear-gradient(135deg, white 0%, var(--pierre-blue-50) 100%);
  border: 1px solid var(--pierre-blue-200);
  border-radius: 0.75rem;
  padding: 1.5rem;
  text-align: center;
  transition: transform 0.2s ease;
}

.pierre-stat-card:hover {
  transform: translateY(-2px);
}

.pierre-stat-value {
  font-size: var(--text-3xl);
  font-weight: var(--font-bold);
  color: var(--pierre-blue-600);
  line-height: 1.2;
}

.pierre-stat-label {
  font-size: var(--text-sm);
  color: var(--pierre-gray-600);
  margin-top: 0.5rem;
}
```

### Button Components

#### Primary Button
```css
.pierre-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.75rem 1.5rem;
  border-radius: 0.5rem;
  font-weight: var(--font-medium);
  font-size: var(--text-sm);
  line-height: 1;
  transition: all 0.2s ease;
  cursor: pointer;
  border: none;
  text-decoration: none;
}

.pierre-btn-primary {
  background: var(--pierre-blue-600);
  color: white;
  box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
}

.pierre-btn-primary:hover {
  background: var(--pierre-blue-700);
  transform: translateY(-1px);
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
}

.pierre-btn-secondary {
  background: white;
  color: var(--pierre-gray-700);
  border: 1px solid var(--pierre-gray-300);
}

.pierre-btn-secondary:hover {
  background: var(--pierre-gray-50);
  border-color: var(--pierre-gray-400);
}

.pierre-btn-danger {
  background: var(--pierre-red-600);
  color: white;
}

.pierre-btn-danger:hover {
  background: var(--pierre-red-700);
}
```

#### Button Sizes
```css
.pierre-btn-sm {
  padding: 0.5rem 1rem;
  font-size: var(--text-xs);
}

.pierre-btn-lg {
  padding: 1rem 2rem;
  font-size: var(--text-base);
}
```

### Form Components

#### Input Fields
```css
.pierre-input {
  width: 100%;
  padding: 0.75rem 1rem;
  border: 1px solid var(--pierre-gray-300);
  border-radius: 0.5rem;
  font-size: var(--text-sm);
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
}

.pierre-input:focus {
  outline: none;
  border-color: var(--pierre-blue-500);
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
}

.pierre-label {
  display: block;
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  color: var(--pierre-gray-700);
  margin-bottom: 0.5rem;
}

.pierre-help-text {
  font-size: var(--text-xs);
  color: var(--pierre-gray-500);
  margin-top: 0.25rem;
}
```

### Status Components

#### Badges
```css
.pierre-badge {
  display: inline-flex;
  align-items: center;
  padding: 0.25rem 0.75rem;
  border-radius: 9999px;
  font-size: var(--text-xs);
  font-weight: var(--font-medium);
  line-height: 1;
}

.pierre-badge-success {
  background: var(--pierre-green-100);
  color: var(--pierre-green-800);
}

.pierre-badge-warning {
  background: var(--pierre-yellow-100);
  color: var(--pierre-yellow-800);
}

.pierre-badge-error {
  background: var(--pierre-red-100);
  color: var(--pierre-red-800);
}

.pierre-badge-info {
  background: var(--pierre-blue-100);
  color: var(--pierre-blue-800);
}
```

#### Status Indicators
```css
.pierre-status-dot {
  width: 0.5rem;
  height: 0.5rem;
  border-radius: 50%;
  display: inline-block;
  margin-right: 0.5rem;
}

.pierre-status-online {
  background: var(--pierre-green-500);
  animation: pulse 2s infinite;
}

.pierre-status-offline {
  background: var(--pierre-gray-400);
}

.pierre-status-error {
  background: var(--pierre-red-500);
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
```

### Navigation Components

#### Header
```css
.pierre-header {
  background: white;
  border-bottom: 1px solid var(--pierre-gray-200);
  box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1);
}

.pierre-header-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 0;
}

.pierre-logo {
  display: flex;
  align-items: center;
  font-weight: var(--font-bold);
  font-size: var(--text-xl);
  color: var(--pierre-blue-600);
  text-decoration: none;
}

.pierre-logo-icon {
  margin-right: 0.5rem;
  font-size: 1.5rem;
}
```

#### Tabs
```css
.pierre-tabs {
  border-bottom: 1px solid var(--pierre-gray-200);
  display: flex;
  gap: 2rem;
}

.pierre-tab {
  padding: 1rem 0;
  border-bottom: 2px solid transparent;
  color: var(--pierre-gray-500);
  text-decoration: none;
  font-weight: var(--font-medium);
  transition: all 0.2s ease;
}

.pierre-tab:hover {
  color: var(--pierre-gray-700);
  border-bottom-color: var(--pierre-gray-300);
}

.pierre-tab-active {
  color: var(--pierre-blue-600);
  border-bottom-color: var(--pierre-blue-600);
}
```

---

## Iconography

### Icon System
- **Primary Icons**: Use consistent icon library (Heroicons, Lucide, or similar)
- **Status Icons**: Emoji for quick recognition (✅❌⚠️📊🔑)
- **Navigation Icons**: Simple, outlined style
- **Action Icons**: Filled style for buttons

### Icon Guidelines
```css
.pierre-icon {
  width: 1rem;
  height: 1rem;
  display: inline-block;
  vertical-align: middle;
}

.pierre-icon-sm { width: 0.75rem; height: 0.75rem; }
.pierre-icon-lg { width: 1.5rem; height: 1.5rem; }
.pierre-icon-xl { width: 2rem; height: 2rem; }
```

---

## Spacing System

### Base Scale (rem)
```css
--space-1:  0.25rem;  /* 4px */
--space-2:  0.5rem;   /* 8px */
--space-3:  0.75rem;  /* 12px */
--space-4:  1rem;     /* 16px */
--space-5:  1.25rem;  /* 20px */
--space-6:  1.5rem;   /* 24px */
--space-8:  2rem;     /* 32px */
--space-10: 2.5rem;   /* 40px */
--space-12: 3rem;     /* 48px */
--space-16: 4rem;     /* 64px */
--space-20: 5rem;     /* 80px */
--space-24: 6rem;     /* 96px */
```

### Component Spacing
- **Card padding**: 1.5rem (24px)
- **Button padding**: 0.75rem 1.5rem (12px 24px)
- **Section gaps**: 2rem (32px)
- **Grid gaps**: 1.5rem (24px)

---

## Animation & Transitions

### Timing Functions
```css
--ease-in:     cubic-bezier(0.4, 0, 1, 1);
--ease-out:    cubic-bezier(0, 0, 0.2, 1);
--ease-in-out: cubic-bezier(0.4, 0, 0.2, 1);
```

### Standard Transitions
```css
--transition-fast:   0.15s ease-out;
--transition-base:   0.2s ease-out;
--transition-slow:   0.3s ease-out;
```

### Hover Effects
- **Buttons**: Slight elevation (2px translateY)
- **Cards**: Enhanced shadow
- **Links**: Color transition
- **Icons**: Scale (1.05x)

---

## Responsive Design

### Breakpoints
```css
--screen-sm:  640px;   /* Small devices */
--screen-md:  768px;   /* Medium devices */
--screen-lg:  1024px;  /* Large devices */
--screen-xl:  1280px;  /* Extra large devices */
--screen-2xl: 1536px;  /* Ultra wide */
```

### Mobile-First Approach
- Design for mobile first
- Progressive enhancement for larger screens
- Touch-friendly interactive elements (min 44px)
- Readable text at all sizes

---

## Accessibility

### Color Contrast
- **Normal text**: 4.5:1 contrast ratio minimum
- **Large text**: 3:1 contrast ratio minimum
- **Interactive elements**: Clear focus states

### Interactive Elements
- **Focus indicators**: Visible outline or shadow
- **Touch targets**: Minimum 44px for mobile
- **Alt text**: For all meaningful images
- **Semantic HTML**: Proper heading hierarchy

---

## Usage Guidelines

### Do's
✅ Use consistent spacing from the scale
✅ Maintain color hierarchy for importance
✅ Follow the component patterns
✅ Test on multiple screen sizes
✅ Ensure sufficient color contrast

### Don'ts
❌ Mix different border radius values
❌ Use colors outside the defined palette
❌ Override component styles arbitrarily
❌ Ignore responsive breakpoints
❌ Skip accessibility considerations

---

## Implementation Strategy

### Phase 1: Foundation
1. Create CSS custom properties file
2. Build base component classes
3. Update existing frontend components

### Phase 2: Component Library
1. Create reusable component library
2. Document all components with examples
3. Build Storybook or similar documentation

### Phase 3: Admin Service
1. Apply design system to admin service
2. Create shared components
3. Ensure brand consistency

### Phase 4: Documentation
1. Create comprehensive style guide
2. Build design system documentation
3. Create usage examples and templates