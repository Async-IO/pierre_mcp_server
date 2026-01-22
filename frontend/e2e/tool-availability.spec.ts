// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for Tool Availability management in admin dashboard.
// ABOUTME: Tests tool listing, search/filter, enable/disable toggles, bulk actions, and overrides.

import { test, expect, type Page } from "@playwright/test";
import {
  setupDashboardMocks,
  loginToDashboard,
  navigateToTab,
} from "./test-helpers";

// Mock tenant tools data - represents effective tools for a tenant
const mockTenantTools = {
  success: true,
  message: "Effective tools retrieved successfully",
  data: [
    {
      tool_name: "get_activities",
      display_name: "Get Activities",
      description: "Retrieve fitness activities from connected providers",
      category: "Activity",
      is_enabled: true,
      source: "Default",
      min_plan: "Free",
    },
    {
      tool_name: "analyze_activity",
      display_name: "Analyze Activity",
      description: "Perform deep analysis on a single activity",
      category: "Activity",
      is_enabled: true,
      source: "Default",
      min_plan: "Professional",
    },
    {
      tool_name: "get_athlete",
      display_name: "Get Athlete Profile",
      description: "Retrieve athlete profile information",
      category: "Profile",
      is_enabled: false,
      source: "TenantOverride",
      min_plan: "Free",
    },
    {
      tool_name: "calculate_metrics",
      display_name: "Calculate Metrics",
      description: "Calculate advanced fitness metrics like TRIMP and TSS",
      category: "Analytics",
      is_enabled: false,
      source: "GlobalDisabled",
      min_plan: "Enterprise",
    },
    {
      tool_name: "connect_provider",
      display_name: "Connect Provider",
      description: "Connect a fitness provider via OAuth",
      category: "Connectivity",
      is_enabled: true,
      source: "Default",
      min_plan: "Free",
    },
    {
      tool_name: "get_stats",
      display_name: "Get Statistics",
      description: "Retrieve athlete statistics and totals",
      category: "Analytics",
      is_enabled: false,
      source: "PlanRestriction",
      min_plan: "Enterprise",
    },
  ],
};

// Mock global disabled tools
const mockGlobalDisabled = {
  success: true,
  message: "Global disabled tools retrieved",
  data: {
    disabled_tools: ["calculate_metrics"],
    count: 1,
  },
};

// Mock availability summary
const mockAvailabilitySummary = {
  success: true,
  message: "Tool availability summary",
  data: {
    tenant_id: "user-123",
    total_tools: 6,
    enabled_tools: 3,
    disabled_tools: 3,
    overridden_tools: 1,
    globally_disabled_count: 1,
    plan_restricted_count: 1,
  },
};

// Minimal config catalog mock to allow AdminConfiguration to load
const mockConfigCatalog = {
  success: true,
  data: {
    total_parameters: 0,
    runtime_configurable_count: 0,
    categories: [],
  },
};

async function setupToolAvailabilityMocks(page: Page) {
  // Set up base dashboard mocks with admin role
  await setupDashboardMocks(page, { role: "admin" });

  // Mock configuration catalog endpoint (required for AdminConfiguration to load)
  await page.route(/\/api\/admin\/config\/catalog/, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(mockConfigCatalog),
    });
  });

  // Mock configuration audit log endpoint
  await page.route(/\/api\/admin\/config\/audit/, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        success: true,
        data: { entries: [], total_count: 0 },
      }),
    });
  });

  // Mock global disabled tools endpoint
  await page.route("**/api/admin/tools/global-disabled", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(mockGlobalDisabled),
    });
  });

  // Mock availability summary endpoint (MUST be before tenant tools to take precedence)
  await page.route("**/api/admin/tools/tenant/*/summary", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(mockAvailabilitySummary),
    });
  });

  // Mock tenant tools endpoint (returns effective tools for a tenant)
  // Uses regex to match /api/admin/tools/tenant/<id> but not /summary or /override
  await page.route(/\/api\/admin\/tools\/tenant\/[^/]+$/, async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(mockTenantTools),
      });
    } else {
      await route.fallback();
    }
  });

  // Mock set override endpoint
  await page.route("**/api/admin/tools/tenant/*/override", async (route) => {
    if (route.request().method() === "POST") {
      const body = route.request().postDataJSON();
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          success: true,
          message: "Tool override set successfully",
          data: {
            tool_name: body.tool_name,
            tenant_id: "user-123",
            is_enabled: body.is_enabled,
            created_by: "admin-123",
            reason: body.reason,
            created_at: new Date().toISOString(),
          },
        }),
      });
    } else {
      await route.fallback();
    }
  });

  // Mock remove override endpoint
  await page.route("**/api/admin/tools/tenant/*/override/*", async (route) => {
    if (route.request().method() === "DELETE") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          success: true,
          message: "Tool override removed successfully",
        }),
      });
    } else {
      await route.fallback();
    }
  });
}

async function navigateToToolAvailability(page: Page) {
  await loginToDashboard(page);
  await page.waitForSelector("nav", { timeout: 10000 });
  await navigateToTab(page, "Configuration");
  await page.waitForSelector('h1:has-text("Configuration Management")', {
    timeout: 10000,
  });
  // Click on Tool Availability tab
  await page.getByRole("tab", { name: "Tool Availability" }).click();
  await page.waitForTimeout(500);
}

test.describe("Tool Availability - Loading and Display", () => {
  test("displays tool availability header and summary stats", async ({
    page,
  }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Check summary cards
    await expect(page.getByText("6").first()).toBeVisible(); // Total Tools
    await expect(page.getByText("Total Tools")).toBeVisible();
    await expect(page.getByText("Enabled").first()).toBeVisible();
    await expect(page.getByText("Disabled").first()).toBeVisible();
    await expect(page.getByText("Overrides")).toBeVisible();
    await expect(page.getByText("Global Blocks")).toBeVisible();
  });

  test("displays globally disabled tools banner", async ({ page }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Check global disabled banner
    await expect(page.getByText("Globally Disabled Tools")).toBeVisible();
    await expect(page.getByText("1 tool(s) are disabled via")).toBeVisible();
    // Use first() to target the badge in the banner, not the code element in the table
    await expect(page.getByText("calculate_metrics").first()).toBeVisible();
  });

  test("displays tools table with all columns", async ({ page }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Check table headers
    await expect(page.getByText("Tool").first()).toBeVisible();
    await expect(page.getByText("Category").first()).toBeVisible();
    await expect(page.getByText("Status").first()).toBeVisible();
    await expect(page.getByText("Source").first()).toBeVisible();
    await expect(page.getByText("Actions").first()).toBeVisible();
  });

  test("displays tool entries with correct information", async ({ page }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Check tool entries
    await expect(page.getByText("Get Activities")).toBeVisible();
    await expect(page.getByText("get_activities")).toBeVisible();
    await expect(page.getByText("Analyze Activity")).toBeVisible();
    await expect(page.getByText("Get Athlete Profile")).toBeVisible();
    await expect(page.getByText("Calculate Metrics")).toBeVisible();
  });

  test("displays correct source badges", async ({ page }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Check source badges - use exact match to avoid matching similar text
    await expect(page.getByText("Default").first()).toBeVisible();
    await expect(page.getByText("Override", { exact: true })).toBeVisible(); // TenantOverride
    await expect(page.getByText("Global Block", { exact: true })).toBeVisible(); // GlobalDisabled
    await expect(page.getByText("Plan Restricted")).toBeVisible(); // PlanRestriction
  });

  test("displays category filter chips", async ({ page }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Check category filter chips - use main section to exclude sidebar nav buttons
    const mainSection = page.getByRole("main");
    await expect(
      mainSection.getByRole("button", { name: "All" }),
    ).toBeVisible();
    await expect(
      mainSection.getByRole("button", { name: "Activity" }),
    ).toBeVisible();
    await expect(
      mainSection.getByRole("button", { name: "Profile" }),
    ).toBeVisible();
    await expect(
      mainSection.getByRole("button", { name: "Analytics" }),
    ).toBeVisible();
    await expect(
      mainSection.getByRole("button", { name: "Connectivity" }),
    ).toBeVisible();
  });
});

test.describe("Tool Availability - Search and Filter", () => {
  test("search filters tools by name", async ({ page }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Wait for table data to load first
    await expect(page.getByText("Get Activities")).toBeVisible({
      timeout: 10000,
    });

    // Type in search box - use "activ" to match both "activities" and "activity"
    const searchInput = page.getByPlaceholder("Search tools...");
    await searchInput.fill("activ");
    await page.waitForTimeout(300);

    // Should show matching tools (both contain "activ" in name)
    await expect(page.getByText("Get Activities")).toBeVisible();
    await expect(page.getByText("Analyze Activity")).toBeVisible();
    // Should not show non-matching tools
    await expect(page.getByText("Get Athlete Profile")).not.toBeVisible();
  });

  test("search filters tools by description", async ({ page }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    const searchInput = page.getByPlaceholder("Search tools...");
    await searchInput.fill("OAuth");

    // Should show matching tool
    await expect(page.getByText("Connect Provider")).toBeVisible();
    // Should not show non-matching tools
    await expect(page.getByText("Get Activities")).not.toBeVisible();
  });

  test("category filter shows only tools in category", async ({ page }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Click Analytics category - use main section to target filter chip, not sidebar
    const mainSection = page.getByRole("main");
    await mainSection.getByRole("button", { name: "Analytics" }).click();
    await page.waitForTimeout(300);

    // Should show Analytics tools
    await expect(page.getByText("Calculate Metrics")).toBeVisible();
    await expect(page.getByText("Get Statistics")).toBeVisible();
    // Should not show other category tools
    await expect(page.getByText("Get Activities")).not.toBeVisible();
  });

  test("clear search restores all tools", async ({ page }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    const searchInput = page.getByPlaceholder("Search tools...");
    await searchInput.fill("activity");

    // Clear search
    await page.getByLabel("Clear search").click();

    // All tools should be visible
    await expect(page.getByText("Get Activities")).toBeVisible();
    await expect(page.getByText("Get Athlete Profile")).toBeVisible();
    await expect(page.getByText("Calculate Metrics")).toBeVisible();
  });

  test("shows no results message when search has no matches", async ({
    page,
  }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    const searchInput = page.getByPlaceholder("Search tools...");
    await searchInput.fill("nonexistent xyz 12345");

    // Should show no results message
    await expect(
      page.getByText("No tools found matching your criteria"),
    ).toBeVisible();
  });
});

test.describe("Tool Availability - Tool Toggle Actions", () => {
  test("clicking toggle on enabled tool shows disable modal", async ({
    page,
  }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Find and click toggle for an enabled tool (Get Activities)
    const getActivitiesRow = page
      .locator("tr")
      .filter({ hasText: "Get Activities" });
    const toggle = getActivitiesRow.locator('[role="switch"]');
    await toggle.click();

    // Modal should appear
    await expect(page.getByText("Confirm Disable Tool(s)")).toBeVisible();
    await expect(
      page.getByText('You are about to disable "get_activities"'),
    ).toBeVisible();
  });

  test("clicking toggle on disabled tool shows enable modal", async ({
    page,
  }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Find and click toggle for a disabled tool (Get Athlete Profile - has TenantOverride)
    const athleteRow = page
      .locator("tr")
      .filter({ hasText: "Get Athlete Profile" });
    const toggle = athleteRow.locator('[role="switch"]');
    await toggle.click();

    // Modal should appear
    await expect(page.getByText("Confirm Enable Tool(s)")).toBeVisible();
    await expect(
      page.getByText('You are about to enable "get_athlete"'),
    ).toBeVisible();
  });

  test("confirming disable calls set override API", async ({ page }) => {
    await setupToolAvailabilityMocks(page);

    let overrideCalled = false;
    let requestBody = null;
    await page.route("**/api/admin/tools/tenant/*/override", async (route) => {
      if (route.request().method() === "POST") {
        overrideCalled = true;
        requestBody = route.request().postDataJSON();
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            success: true,
            message: "Tool override set successfully",
            data: {
              tool_name: requestBody.tool_name,
              tenant_id: "user-123",
              is_enabled: requestBody.is_enabled,
              created_by: "admin-123",
              reason: requestBody.reason,
              created_at: new Date().toISOString(),
            },
          }),
        });
      } else {
        await route.fallback();
      }
    });

    await navigateToToolAvailability(page);

    // Click toggle for enabled tool
    const getActivitiesRow = page
      .locator("tr")
      .filter({ hasText: "Get Activities" });
    const toggle = getActivitiesRow.locator('[role="switch"]');
    await toggle.click();

    // Fill reason and confirm
    await page.getByPlaceholder(/e.g.,/).fill("Test disable reason");
    await page.getByRole("button", { name: "Disable" }).click();

    await page.waitForTimeout(500);
    expect(overrideCalled).toBe(true);
    expect(requestBody.is_enabled).toBe(false);
    expect(requestBody.reason).toBe("Test disable reason");
  });

  test("globally disabled tools cannot be toggled", async ({ page }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Find the globally disabled tool row (Calculate Metrics)
    const metricsRow = page
      .locator("tr")
      .filter({ hasText: "Calculate Metrics" });

    // Toggle should be disabled
    const toggle = metricsRow.locator('[role="switch"]');
    await expect(toggle).toBeVisible();

    // Should have opacity and cursor-not-allowed (indicates disabled state)
    await expect(metricsRow).toHaveClass(/opacity-60/);
  });

  test("cancel button closes modal without making changes", async ({
    page,
  }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Click toggle for enabled tool
    const getActivitiesRow = page
      .locator("tr")
      .filter({ hasText: "Get Activities" });
    const toggle = getActivitiesRow.locator('[role="switch"]');
    await toggle.click();

    // Modal should appear
    await expect(page.getByText("Confirm Disable Tool(s)")).toBeVisible();

    // Click cancel
    await page.getByRole("button", { name: "Cancel" }).click();

    // Modal should close
    await expect(page.getByText("Confirm Disable Tool(s)")).not.toBeVisible();
  });
});

test.describe("Tool Availability - Bulk Actions", () => {
  test("selecting tools shows bulk action buttons", async ({ page }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Select a tool checkbox
    const getActivitiesRow = page
      .locator("tr")
      .filter({ hasText: "Get Activities" });
    const checkbox = getActivitiesRow.locator('input[type="checkbox"]');
    await checkbox.check();

    // Bulk action buttons should appear
    await expect(page.getByText("1 selected")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Enable Selected" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Disable Selected" }),
    ).toBeVisible();
  });

  test("select all checkbox selects non-globally-disabled tools", async ({
    page,
  }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Click select all checkbox
    const selectAllCheckbox = page.locator('thead input[type="checkbox"]');
    await selectAllCheckbox.check();

    // Should show 5 selected (6 tools - 1 globally disabled = 5 selectable)
    await expect(page.getByText("5 selected")).toBeVisible();
  });

  test("bulk disable shows modal with count", async ({ page }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Select two tools
    const getActivitiesRow = page
      .locator("tr")
      .filter({ hasText: "Get Activities" });
    const analyzeRow = page
      .locator("tr")
      .filter({ hasText: "Analyze Activity" });

    await getActivitiesRow.locator('input[type="checkbox"]').check();
    await analyzeRow.locator('input[type="checkbox"]').check();

    // Click bulk disable
    await page.getByRole("button", { name: "Disable Selected" }).click();

    // Modal should show bulk count
    await expect(page.getByText("Confirm Disable Tool(s)")).toBeVisible();
    await expect(
      page.getByText("You are about to disable 2 tool(s)"),
    ).toBeVisible();
  });

  test("bulk enable shows modal with count", async ({ page }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Select disabled tool
    const athleteRow = page
      .locator("tr")
      .filter({ hasText: "Get Athlete Profile" });
    await athleteRow.locator('input[type="checkbox"]').check();

    // Click bulk enable
    await page.getByRole("button", { name: "Enable Selected" }).click();

    // Modal should show bulk count
    await expect(page.getByText("Confirm Enable Tool(s)")).toBeVisible();
    await expect(
      page.getByText("You are about to enable 1 tool(s)"),
    ).toBeVisible();
  });
});

test.describe("Tool Availability - Override Management", () => {
  test("remove override button visible only for overridden tools", async ({
    page,
  }) => {
    await setupToolAvailabilityMocks(page);
    await navigateToToolAvailability(page);

    // Get Athlete Profile has TenantOverride - should have remove button
    const athleteRow = page
      .locator("tr")
      .filter({ hasText: "Get Athlete Profile" });
    const removeButton = athleteRow.locator(
      'button[title="Remove override (revert to default)"]',
    );
    await expect(removeButton).toBeVisible();

    // Get Activities has Default source - should NOT have remove button
    const activitiesRow = page
      .locator("tr")
      .filter({ hasText: "Get Activities" });
    const activitiesRemoveButton = activitiesRow.locator(
      'button[title="Remove override (revert to default)"]',
    );
    await expect(activitiesRemoveButton).not.toBeVisible();
  });

  test("clicking remove override calls delete API", async ({ page }) => {
    await setupToolAvailabilityMocks(page);

    let deleteCalled = false;
    let deleteUrl = "";
    await page.route(
      "**/api/admin/tools/tenant/*/override/*",
      async (route) => {
        if (route.request().method() === "DELETE") {
          deleteCalled = true;
          deleteUrl = route.request().url();
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify({
              success: true,
              message: "Tool override removed successfully",
            }),
          });
        } else {
          await route.fallback();
        }
      },
    );

    await navigateToToolAvailability(page);

    // Click remove override for Get Athlete Profile
    const athleteRow = page
      .locator("tr")
      .filter({ hasText: "Get Athlete Profile" });
    const removeButton = athleteRow.locator(
      'button[title="Remove override (revert to default)"]',
    );
    await removeButton.click();

    await page.waitForTimeout(500);
    expect(deleteCalled).toBe(true);
    expect(deleteUrl).toContain("get_athlete");
  });
});

test.describe("Tool Availability - Error Handling", () => {
  test("shows error message when tenant tools fail to load", async ({
    page,
  }) => {
    await setupDashboardMocks(page, { role: "admin" });

    // Mock configuration catalog endpoint (required for AdminConfiguration to load)
    await page.route(/\/api\/admin\/config\/catalog/, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(mockConfigCatalog),
      });
    });

    // Mock configuration audit log endpoint
    await page.route(/\/api\/admin\/config\/audit/, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          success: true,
          data: { entries: [], total_count: 0 },
        }),
      });
    });

    // Mock failing tenant tools endpoint
    await page.route("**/api/admin/tools/tenant/*", async (route) => {
      await route.fulfill({
        status: 500,
        contentType: "application/json",
        body: JSON.stringify({ error: "Internal server error" }),
      });
    });

    // Mock summary endpoint to also fail
    await page.route("**/api/admin/tools/tenant/*/summary", async (route) => {
      await route.fulfill({
        status: 500,
        contentType: "application/json",
        body: JSON.stringify({ error: "Internal server error" }),
      });
    });

    // Mock global disabled (can succeed)
    await page.route("**/api/admin/tools/global-disabled", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(mockGlobalDisabled),
      });
    });

    await loginToDashboard(page);
    await page.waitForSelector("nav", { timeout: 10000 });
    await navigateToTab(page, "Configuration");
    await page.waitForSelector('h1:has-text("Configuration Management")', {
      timeout: 10000,
    });
    await page.getByRole("tab", { name: "Tool Availability" }).click();

    // Should show error message
    await expect(
      page.getByText("Failed to load tool availability"),
    ).toBeVisible({ timeout: 10000 });
  });

  test("shows error in modal when set override fails", async ({ page }) => {
    await setupToolAvailabilityMocks(page);

    // Override the set override endpoint to fail
    await page.route("**/api/admin/tools/tenant/*/override", async (route) => {
      if (route.request().method() === "POST") {
        await route.fulfill({
          status: 500,
          contentType: "application/json",
          body: JSON.stringify({ error: "Failed to set override" }),
        });
      } else {
        await route.fallback();
      }
    });

    await navigateToToolAvailability(page);

    // Click toggle for enabled tool
    const getActivitiesRow = page
      .locator("tr")
      .filter({ hasText: "Get Activities" });
    const toggle = getActivitiesRow.locator('[role="switch"]');
    await toggle.click();

    // Confirm disable
    await page.getByRole("button", { name: "Disable" }).click();

    // Should show error in modal
    await expect(page.getByText("Failed to update tool settings")).toBeVisible({
      timeout: 5000,
    });
  });
});

test.describe("Tool Availability - Access Control", () => {
  test("non-admin users cannot see Configuration tab", async ({ page }) => {
    await setupDashboardMocks(page, { role: "user" });

    // Mock endpoints for regular users (can fail or be empty)
    await page.route("**/api/admin/tools/**", async (route) => {
      await route.fulfill({
        status: 403,
        contentType: "application/json",
        body: JSON.stringify({ error: "Forbidden" }),
      });
    });

    await loginToDashboard(page);
    // Non-admin users see chat-first layout (no sidebar nav)
    await page.waitForSelector("main", { timeout: 10000 });

    // Non-admin users see the ChatTab interface with settings button, not admin sidebar
    await expect(page.locator('button[title="Open settings"]')).toBeVisible();
    // Non-admin users see ChatTab sidebar (nav), but NOT admin-specific tabs like Configuration
    await expect(page.locator('button').filter({ has: page.locator('span:has-text("Configuration")') })).not.toBeVisible();
  });

  test("admin users can see Tool Availability tab", async ({ page }) => {
    await setupToolAvailabilityMocks(page);
    await loginToDashboard(page);
    await page.waitForSelector("nav", { timeout: 10000 });

    // Navigate to Configuration
    await navigateToTab(page, "Configuration");
    await page.waitForSelector('h1:has-text("Configuration Management")', {
      timeout: 10000,
    });

    // Tool Availability tab should be visible
    await expect(
      page.getByRole("tab", { name: "Tool Availability" }),
    ).toBeVisible();
  });
});
