/**
 * Issue #69 — Frontend Form Validation Tests
 *
 * Covers:
 *  - Required field checks
 *  - Email / password pattern matching
 *  - Real-time validation (onChange / onBlur)
 *  - Submission blocked when form state is invalid
 *  - UI recovery once errors are corrected
 *  - Idempotency (form is not submitted twice)
 *
 * Stack: React · Vitest · React Testing Library · @testing-library/user-event
 *
 * NOTE: Adjust the import path below to match your actual component location.
 *       e.g. import { RegistrationForm } from '../../src/components/forms/RegistrationForm'
 */

import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';

// ─── Adjust this import to your actual component path ────────────────────────
import { RegistrationForm } from '../../src/components/forms/RegistrationForm';
// ─────────────────────────────────────────────────────────────────────────────

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Renders the form with a fresh vi.fn() onSubmit mock and returns both. */
function setup() {
  const onSubmit = vi.fn();
  const utils = render(<RegistrationForm onSubmit={onSubmit} />);
  return { onSubmit, ...utils };
}

/** Convenience: grab the three main fields and the submit button. */
function getFields() {
  return {
    emailInput: screen.getByLabelText(/email/i),
    passwordInput: screen.getByLabelText(/^password$/i),
    confirmPasswordInput: screen.getByLabelText(/confirm password/i),
    submitButton: screen.getByRole('button', { name: /register|sign up|submit/i }),
  };
}

// ---------------------------------------------------------------------------
// Test Suite
// ---------------------------------------------------------------------------

describe('RegistrationForm — validation', () => {
  let user: ReturnType<typeof userEvent.setup>;

  beforeEach(() => {
    // userEvent.setup() gives us a realistic browser-like event simulation
    user = userEvent.setup();
  });

  // ── 1. Required field checks ─────────────────────────────────────────────

  describe('required field checks', () => {
    it('shows an error for each empty required field on submit attempt', async () => {
      // Arrange
      const { onSubmit } = setup();
      const { submitButton } = getFields();

      // Act — submit without filling anything
      await user.click(submitButton);

      // Assert
      expect(await screen.findByText(/email is required/i)).toBeInTheDocument();
      expect(await screen.findByText(/password is required/i)).toBeInTheDocument();
      expect(onSubmit).not.toHaveBeenCalled();
    });

    it('does not show required errors before the user interacts with the form', () => {
      // Arrange
      setup();

      // Assert — no error messages on initial render
      expect(screen.queryByText(/is required/i)).not.toBeInTheDocument();
    });
  });

  // ── 2. Email pattern matching ─────────────────────────────────────────────

  describe('email validation', () => {
    it('shows an error for an invalid email format', async () => {
      // Arrange
      setup();
      const { emailInput } = getFields();

      // Act
      await user.type(emailInput, 'not-an-email');
      await user.tab(); // trigger onBlur

      // Assert
      expect(await screen.findByText(/invalid email/i)).toBeInTheDocument();
    });

    it('clears the email error when a valid email is entered', async () => {
      // Arrange
      setup();
      const { emailInput } = getFields();

      // Act — first enter invalid, then correct it
      await user.type(emailInput, 'bad-email');
      await user.tab();
      await screen.findByText(/invalid email/i); // wait for error to appear

      await user.clear(emailInput);
      await user.type(emailInput, 'player@chess.com');
      await user.tab();

      // Assert — error should be gone
      await waitFor(() =>
        expect(screen.queryByText(/invalid email/i)).not.toBeInTheDocument()
      );
    });

    it('accepts a valid email without showing an error', async () => {
      // Arrange
      setup();
      const { emailInput } = getFields();

      // Act
      await user.type(emailInput, 'valid@example.com');
      await user.tab();

      // Assert
      expect(screen.queryByText(/invalid email/i)).not.toBeInTheDocument();
    });
  });

  // ── 3. Password pattern matching ──────────────────────────────────────────

  describe('password validation', () => {
    it('shows an error when password is shorter than the minimum length', async () => {
      // Arrange
      setup();
      const { passwordInput } = getFields();

      // Act
      await user.type(passwordInput, 'short');
      await user.tab();

      // Assert
      expect(
        await screen.findByText(/password must be at least/i)
      ).toBeInTheDocument();
    });

    it('shows an error when password lacks required complexity', async () => {
      // Arrange
      setup();
      const { passwordInput } = getFields();

      // Act — all lowercase, no numbers/symbols
      await user.type(passwordInput, 'alllowercase');
      await user.tab();

      // Assert
      expect(
        await screen.findByText(/password must contain/i)
      ).toBeInTheDocument();
    });

    it('accepts a strong password without showing an error', async () => {
      // Arrange
      setup();
      const { passwordInput } = getFields();

      // Act
      await user.type(passwordInput, 'Str0ng!Pass');
      await user.tab();

      // Assert
      expect(screen.queryByText(/password must/i)).not.toBeInTheDocument();
    });

    it('shows an error when confirm password does not match', async () => {
      // Arrange
      setup();
      const { passwordInput, confirmPasswordInput } = getFields();

      // Act
      await user.type(passwordInput, 'Str0ng!Pass');
      await user.type(confirmPasswordInput, 'DifferentPass1!');
      await user.tab();

      // Assert
      expect(await screen.findByText(/passwords do not match/i)).toBeInTheDocument();
    });
  });

  // ── 4. Real-time validation (onChange / onBlur) ───────────────────────────

  describe('real-time validation', () => {
    it('shows an email error immediately after the field loses focus (onBlur)', async () => {
      // Arrange
      setup();
      const { emailInput } = getFields();

      // Act
      await user.type(emailInput, 'oops');
      await user.tab(); // blur

      // Assert — error appears without needing to submit
      expect(await screen.findByText(/invalid email/i)).toBeInTheDocument();
    });

    it('updates the password error in real-time as the user types (onChange)', async () => {
      // Arrange
      setup();
      const { passwordInput } = getFields();

      // Act — type a weak password character by character
      await user.type(passwordInput, 'abc');

      // Assert — error visible while still typing
      expect(
        await screen.findByText(/password must be at least/i)
      ).toBeInTheDocument();

      // Act — continue typing to meet the minimum length
      await user.type(passwordInput, 'Def1!xyz');

      // Assert — length error gone once requirement is met
      await waitFor(() =>
        expect(
          screen.queryByText(/password must be at least/i)
        ).not.toBeInTheDocument()
      );
    });
  });

  // ── 5. Submission blocked when form state is invalid ─────────────────────

  describe('submission blocking', () => {
    it('does not call onSubmit when required fields are empty', async () => {
      // Arrange
      const { onSubmit } = setup();
      const { submitButton } = getFields();

      // Act
      await user.click(submitButton);

      // Assert
      expect(onSubmit).not.toHaveBeenCalled();
    });

    it('does not call onSubmit when the email is invalid', async () => {
      // Arrange
      const { onSubmit } = setup();
      const { emailInput, passwordInput, confirmPasswordInput, submitButton } =
        getFields();

      // Act
      await user.type(emailInput, 'bad-email');
      await user.type(passwordInput, 'Str0ng!Pass');
      await user.type(confirmPasswordInput, 'Str0ng!Pass');
      await user.click(submitButton);

      // Assert
      expect(onSubmit).not.toHaveBeenCalled();
    });

    it('does not call onSubmit when passwords do not match', async () => {
      // Arrange
      const { onSubmit } = setup();
      const { emailInput, passwordInput, confirmPasswordInput, submitButton } =
        getFields();

      // Act
      await user.type(emailInput, 'player@chess.com');
      await user.type(passwordInput, 'Str0ng!Pass');
      await user.type(confirmPasswordInput, 'WrongPass1!');
      await user.click(submitButton);

      // Assert
      expect(onSubmit).not.toHaveBeenCalled();
    });

    it('calls onSubmit exactly once when all fields are valid', async () => {
      // Arrange
      const { onSubmit } = setup();
      const { emailInput, passwordInput, confirmPasswordInput, submitButton } =
        getFields();

      // Act
      await user.type(emailInput, 'player@chess.com');
      await user.type(passwordInput, 'Str0ng!Pass');
      await user.type(confirmPasswordInput, 'Str0ng!Pass');
      await user.click(submitButton);

      // Assert
      await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
      expect(onSubmit).toHaveBeenCalledWith(
        expect.objectContaining({ email: 'player@chess.com' })
      );
    });
  });

  // ── 6. UI recovery once errors are corrected ─────────────────────────────

  describe('error recovery', () => {
    it('removes the email error after the user corrects the value', async () => {
      // Arrange
      setup();
      const { emailInput } = getFields();

      // Act — trigger error
      await user.type(emailInput, 'notvalid');
      await user.tab();
      expect(await screen.findByText(/invalid email/i)).toBeInTheDocument();

      // Act — fix the value
      await user.clear(emailInput);
      await user.type(emailInput, 'fixed@chess.com');
      await user.tab();

      // Assert — error gone
      await waitFor(() =>
        expect(screen.queryByText(/invalid email/i)).not.toBeInTheDocument()
      );
    });

    it('removes the password mismatch error once passwords match', async () => {
      // Arrange
      setup();
      const { passwordInput, confirmPasswordInput } = getFields();

      // Act — trigger mismatch error
      await user.type(passwordInput, 'Str0ng!Pass');
      await user.type(confirmPasswordInput, 'Mismatch1!');
      await user.tab();
      expect(await screen.findByText(/passwords do not match/i)).toBeInTheDocument();

      // Act — fix confirm password
      await user.clear(confirmPasswordInput);
      await user.type(confirmPasswordInput, 'Str0ng!Pass');
      await user.tab();

      // Assert — mismatch error gone
      await waitFor(() =>
        expect(
          screen.queryByText(/passwords do not match/i)
        ).not.toBeInTheDocument()
      );
    });

    it('enables the submit button once all validation errors are resolved', async () => {
      // Arrange
      setup();
      const { emailInput, passwordInput, confirmPasswordInput, submitButton } =
        getFields();

      // Act — fill all fields correctly
      await user.type(emailInput, 'player@chess.com');
      await user.type(passwordInput, 'Str0ng!Pass');
      await user.type(confirmPasswordInput, 'Str0ng!Pass');

      // Assert — submit button is no longer disabled
      await waitFor(() => expect(submitButton).not.toBeDisabled());
    });
  });

  // ── 7. Idempotency — form is not submitted twice ──────────────────────────

  describe('idempotency', () => {
    it('calls onSubmit only once even when the submit button is clicked multiple times rapidly', async () => {
      // Arrange
      const { onSubmit } = setup();
      const { emailInput, passwordInput, confirmPasswordInput, submitButton } =
        getFields();

      // Act — fill valid data
      await user.type(emailInput, 'player@chess.com');
      await user.type(passwordInput, 'Str0ng!Pass');
      await user.type(confirmPasswordInput, 'Str0ng!Pass');

      // Act — double-click / rapid clicks
      await user.click(submitButton);
      await user.click(submitButton);
      await user.click(submitButton);

      // Assert — handler invoked exactly once
      await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    });

    it('disables the submit button after a successful submission to prevent re-submission', async () => {
      // Arrange
      // Simulate an async onSubmit that takes a moment to resolve
      const onSubmit = vi.fn(
        () => new Promise<void>((resolve) => setTimeout(resolve, 50))
      );
      render(<RegistrationForm onSubmit={onSubmit} />);
      const { emailInput, passwordInput, confirmPasswordInput, submitButton } =
        getFields();

      // Act
      await user.type(emailInput, 'player@chess.com');
      await user.type(passwordInput, 'Str0ng!Pass');
      await user.type(confirmPasswordInput, 'Str0ng!Pass');
      await user.click(submitButton);

      // Assert — button is disabled while submission is in-flight
      expect(submitButton).toBeDisabled();

      // Wait for submission to complete
      await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    });
  });
});
