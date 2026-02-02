// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Shared test helpers for mobile visual testing against real backend.
// ABOUTME: Provides authentication helpers and common test utilities.

// Visual test user credentials (created by scripts/visual-test-setup.sh)
const TEST_USERS = {
  mobiletest: {
    email: 'mobiletest@pierre.dev',
    password: 'MobileTest123!',
    displayName: 'Mobile Test User',
  },
  webtest: {
    email: 'webtest@pierre.dev',
    password: 'WebTest123!',
    displayName: 'Web Test User',
  },
};

/**
 * Login to the app with visual test credentials.
 */
async function loginAsMobileTestUser() {
  const user = TEST_USERS.mobiletest;

  await waitFor(element(by.id('login-screen')))
    .toBeVisible()
    .withTimeout(10000);

  await element(by.id('email-input')).clearText();
  await element(by.id('email-input')).typeText(user.email);
  await element(by.id('password-input')).clearText();
  await element(by.id('password-input')).typeText(user.password + '\n');

  await waitFor(element(by.id('login-button')))
    .toBeVisible()
    .withTimeout(5000);
  await element(by.id('login-button')).tap();

  // Wait for login to complete
  await waitFor(element(by.id('login-screen')))
    .not.toBeVisible()
    .withTimeout(15000);
}

/**
 * Navigate to a tab via bottom tab bar.
 * Tab names: 'chat', 'coaches', 'discover', 'insights', 'settings'
 */
async function navigateToTab(tabName) {
  const tabId = `tab-${tabName.toLowerCase()}`;
  await waitFor(element(by.id(tabId)))
    .toBeVisible()
    .withTimeout(5000);
  await element(by.id(tabId)).tap();
}

/**
 * Take a screenshot with consistent naming.
 */
async function takeVisualScreenshot(testName) {
  // Detox takes screenshots automatically on failures
  // For explicit screenshots, use device.takeScreenshot()
  const screenshotName = testName.replace(/\s+/g, '-').toLowerCase();
  await device.takeScreenshot(screenshotName);
}

/**
 * Scroll down to reveal more content.
 */
async function scrollDown(scrollViewId = 'main-scroll-view') {
  await element(by.id(scrollViewId)).swipe('up', 'slow');
}

/**
 * Pull to refresh.
 */
async function pullToRefresh(scrollViewId = 'main-scroll-view') {
  await element(by.id(scrollViewId)).swipe('down', 'fast');
}

/**
 * Check if element is visible without throwing.
 */
async function isVisible(elementMatcher) {
  try {
    await expect(elementMatcher).toBeVisible();
    return true;
  } catch {
    return false;
  }
}

/**
 * Wait for element with custom timeout.
 */
async function waitForVisible(elementMatcher, timeout = 5000) {
  await waitFor(elementMatcher)
    .toBeVisible()
    .withTimeout(timeout);
}

module.exports = {
  TEST_USERS,
  loginAsMobileTestUser,
  navigateToTab,
  takeVisualScreenshot,
  scrollDown,
  pullToRefresh,
  isVisible,
  waitForVisible,
};
