# Pierre Frontend Style Guide

This guide defines the component usage patterns and design system rules for the Pierre admin dashboard (frontend/).

## Component Library

All UI components are in `frontend/src/components/ui/`. Import from the barrel export:

```tsx
import { Button, Card, CardHeader, Badge, Modal, Input, ConfirmDialog } from './ui';
```

## Required Component Usage

### Buttons

**ALWAYS** use the `Button` component for clickable actions.

```tsx
// CORRECT
import { Button } from './ui';

<Button variant="primary" onClick={handleSubmit}>
  Submit
</Button>

<Button variant="secondary" size="sm" onClick={handleCancel}>
  Cancel
</Button>

<Button variant="danger" loading={isDeleting}>
  Delete
</Button>

// WRONG - Never use raw button elements
<button className="bg-pierre-violet text-white px-4 py-2" onClick={...}>
  Submit
</button>
```

**Variants:** `primary`, `gradient`, `secondary`, `danger`, `success`, `outline`
**Sizes:** `sm`, `md` (default), `lg`

### Cards

**ALWAYS** use `Card` for elevated content containers.

```tsx
// CORRECT
import { Card, CardHeader } from './ui';

<Card>
  <CardHeader title="API Keys" subtitle="Manage your API keys" />
  <div className="space-y-4">
    {/* content */}
  </div>
</Card>

// For stat cards
<Card variant="stat">
  <div className="text-3xl font-bold">42</div>
  <div className="text-sm text-pierre-gray-500">Total Requests</div>
</Card>

// WRONG - Never use raw div with border for card-like content
<div className="border border-pierre-gray-200 rounded-lg p-4 bg-white">
  {/* content */}
</div>
```

### Badges

**ALWAYS** use `Badge` for status indicators.

```tsx
// CORRECT
import { Badge } from './ui';

<Badge variant="success">Active</Badge>
<Badge variant="error">Failed</Badge>
<Badge variant="warning">Pending</Badge>
<Badge variant="info">Processing</Badge>

// API tier badges
<Badge variant="trial">Trial</Badge>
<Badge variant="starter">Starter</Badge>
<Badge variant="professional">Professional</Badge>
<Badge variant="enterprise">Enterprise</Badge>

// WRONG - Never use raw spans with colors
<span className="bg-green-100 text-green-800 px-2 py-1 rounded-full text-xs">
  Active
</span>
```

### Loading States

**ALWAYS** use the `pierre-spinner` class.

```tsx
// CORRECT - Loading spinner
<div className="pierre-spinner" />

// CORRECT - Centered loading state
<div className="flex items-center justify-center h-64">
  <div className="pierre-spinner" />
</div>

// CORRECT - Button with loading
<Button variant="primary" loading={isSubmitting}>
  {isSubmitting ? 'Saving...' : 'Save'}
</Button>

// WRONG - Custom inline spinner
<div className="animate-spin rounded-full h-8 w-8 border-b-2 border-pierre-blue-600" />
```

### Forms

Use the CSS classes defined in index.css:

```tsx
// CORRECT
<label className="label">Email Address</label>
<input type="email" className="input-field" placeholder="Enter email" />
<span className="help-text">We'll never share your email.</span>
<span className="error-text">Invalid email format</span>

// Or use Input component if available
import { Input } from './ui';
<Input label="Email" type="email" error={errors.email} />
```

### Modals

**ALWAYS** use the `Modal` component for overlays.

```tsx
// CORRECT
import { Modal, ConfirmDialog } from './ui';

<Modal isOpen={showModal} onClose={() => setShowModal(false)} title="Edit User">
  {/* modal content */}
</Modal>

<ConfirmDialog
  isOpen={showConfirm}
  onClose={() => setShowConfirm(false)}
  onConfirm={handleDelete}
  title="Delete Item"
  message="Are you sure? This cannot be undone."
  variant="danger"
/>

// WRONG - Custom modal implementation
<div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center">
  <div className="bg-white rounded-lg p-6">
    {/* content */}
  </div>
</div>
```

## Color Usage

### Pierre Brand Colors

```tsx
// Primary brand colors (use for main actions, headers)
className="bg-pierre-violet"      // #7C3AED - Intelligence, AI
className="bg-pierre-cyan"        // #06B6D4 - Data flow, connectivity
className="bg-gradient-pierre"    // Violet to Cyan gradient

// Three Pillars (use semantically)
className="bg-pierre-activity"    // #10B981 - Activity/Fitness related
className="bg-pierre-nutrition"   // #F59E0B - Nutrition/Food related
className="bg-pierre-recovery"    // #6366F1 - Recovery/Sleep related
```

### Status Colors

```tsx
// Status indicators (prefer Badge component)
className="text-pierre-green-600"  // Success, connected, active
className="text-pierre-red-600"    // Error, failed, disconnected
className="text-pierre-yellow-600" // Warning, pending
className="text-pierre-blue-600"   // Info, processing
```

### Gray Scale

```tsx
// Text colors
className="text-pierre-gray-900"   // Headings
className="text-pierre-gray-700"   // Body text
className="text-pierre-gray-500"   // Secondary text
className="text-pierre-gray-400"   // Placeholder, disabled

// Backgrounds
className="bg-pierre-gray-50"      // Page background
className="bg-pierre-gray-100"     // Secondary background
className="bg-white"               // Card/content background

// Borders
className="border-pierre-gray-200" // Default borders
className="border-pierre-gray-300" // Input borders
```

## Spacing

Use Tailwind's spacing scale consistently:

```tsx
// Component spacing
className="space-y-4"   // Between list items
className="space-y-6"   // Between sections
className="gap-2"       // Between inline elements
className="gap-4"       // Between grid items

// Padding
className="p-4"         // Small containers
className="p-6"         // Cards, modals
className="px-6 py-4"   // Headers

// Margins
className="mt-4"        // After headings
className="mb-6"        // Before sections
```

## Typography

```tsx
// Headings
className="text-lg font-semibold text-pierre-gray-900"   // Card titles
className="text-xl font-bold text-pierre-gray-900"       // Page titles
className="text-sm font-medium text-pierre-gray-700"     // Labels

// Body text
className="text-sm text-pierre-gray-600"                 // Descriptions
className="text-xs text-pierre-gray-500"                 // Help text

// Code/monospace
className="font-mono text-sm bg-pierre-gray-100 px-2 py-1 rounded"
```

## Common Patterns

### List Item Pattern

```tsx
// CORRECT - Use Card for each item
{items.map((item) => (
  <Card key={item.id} className="p-4">
    <div className="flex items-start justify-between">
      <div className="flex-1">
        <h3 className="text-lg font-medium text-pierre-gray-900">
          {item.name}
        </h3>
        <div className="flex items-center gap-2 mt-2">
          <Badge variant={item.isActive ? 'success' : 'error'}>
            {item.isActive ? 'Active' : 'Inactive'}
          </Badge>
        </div>
      </div>
      <div className="flex gap-2">
        <Button variant="secondary" size="sm">Edit</Button>
        <Button variant="danger" size="sm">Delete</Button>
      </div>
    </div>
  </Card>
))}
```

### Empty State Pattern

```tsx
<div className="text-center py-8 text-pierre-gray-500">
  <div className="text-4xl mb-4">icon-here</div>
  <p className="text-lg mb-2">No items yet</p>
  <p>Create your first item to get started</p>
</div>
```

### Error State Pattern

```tsx
<div className="bg-red-50 border border-red-200 rounded-lg p-6">
  <div className="flex items-center gap-3">
    <svg className="w-6 h-6 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
    <div>
      <h3 className="text-lg font-medium text-red-900">Error Title</h3>
      <p className="text-red-700 mt-1">{error.message}</p>
    </div>
  </div>
</div>
```

### Filter/Action Bar Pattern

```tsx
<div className="flex items-center justify-between">
  <StatusFilter
    value={statusFilter}
    onChange={setStatusFilter}
    activeCount={activeCount}
    inactiveCount={inactiveCount}
    totalCount={totalCount}
  />

  {selectedItems.size > 0 && (
    <div className="flex items-center gap-2">
      <span className="text-sm text-pierre-gray-600">
        {selectedItems.size} selected
      </span>
      <Button variant="secondary" size="sm">
        Bulk Action
      </Button>
    </div>
  )}
</div>
```

## Anti-Patterns to Avoid

### 1. Raw HTML Elements

```tsx
// WRONG
<button onClick={...}>Click me</button>
<div className="border rounded-lg p-4">Card content</div>

// CORRECT
<Button onClick={...}>Click me</Button>
<Card>Card content</Card>
```

### 2. Custom Spinners

```tsx
// WRONG
<div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />

// CORRECT
<div className="pierre-spinner" />
```

### 3. Inline Status Colors

```tsx
// WRONG
<span className="bg-green-100 text-green-800 px-2 py-1 rounded">Active</span>

// CORRECT
<Badge variant="success">Active</Badge>
```

### 4. Non-Design-Token Colors

```tsx
// WRONG
className="bg-[#7C3AED]"  // Raw hex
className="text-purple-600"  // Non-pierre color

// CORRECT
className="bg-pierre-violet"
className="text-pierre-gray-600"
```

### 5. Inconsistent Spacing

```tsx
// WRONG - Mixed spacing values
className="mt-3 mb-5 p-7"

// CORRECT - Use spacing scale
className="mt-4 mb-6 p-6"
```

## File Organization

```
frontend/src/
  components/
    ui/               # Design system components
      Button.tsx
      Card.tsx
      Badge.tsx
      Modal.tsx
      Input.tsx
      index.ts        # Barrel export
    Feature.tsx       # Feature components use ui/
  index.css           # Tailwind + component classes
```

## Checklist Before Committing

- [ ] All buttons use `<Button>` component
- [ ] All cards use `<Card>` component
- [ ] All status indicators use `<Badge>` component
- [ ] Loading states use `pierre-spinner` class
- [ ] No raw hex color values
- [ ] Consistent spacing from Tailwind scale
- [ ] Modals use `<Modal>` or `<ConfirmDialog>`
- [ ] Forms use design system classes
