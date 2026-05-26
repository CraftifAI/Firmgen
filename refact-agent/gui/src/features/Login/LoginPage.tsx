import React, { useState } from "react";
import {
  Flex,
  Box,
  Button,
  Text,
  TextField,
  Container,
  Heading,
  Card,
  Link,
} from "@radix-ui/themes";
import { Mail, Lock, Eye, EyeOff, KeyRound } from "lucide-react";
import { useCraftifAuth } from "../../hooks";
import { useEventsBusForIDE } from "../../hooks";

type SignUpStep = "credentials" | "otp";

export const LoginPage: React.FC = () => {
  const { login, sendSignupOtp, verifySignupOtp, loading, error } = useCraftifAuth();
  const { setupHost } = useEventsBusForIDE();

  const [isSignUp, setIsSignUp] = useState(false);
  const [signUpStep, setSignUpStep] = useState<SignUpStep>("credentials");
  const [showPassword, setShowPassword] = useState(false);

  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [credentialsError, setCredentialsError] = useState<string | null>(null);
  const [otp, setOtp] = useState("");

  const handleToggleMode = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsSignUp(!isSignUp);
    setSignUpStep("credentials");
    setEmail("");
    setPassword("");
    setConfirmPassword("");
    setCredentialsError(null);
    setOtp("");
  };

  const handleSignIn = async (event: React.FormEvent) => {
    event.preventDefault();
    if (loading) return;
    try {
      const token = await login(email, password);
      proceedWithAuth(token);
    } catch {
      // error surfaced from hook
    }
  };

  const handleSendOtp = async (event: React.FormEvent) => {
    event.preventDefault();
    if (loading) return;
    if (password !== confirmPassword) {
      setCredentialsError("Passwords do not match.");
      return;
    }
    setCredentialsError(null);
    try {
      await sendSignupOtp(email, password);
      setSignUpStep("otp");
    } catch {
      // error surfaced from hook
    }
  };

  const handleVerifyOtp = async (event: React.FormEvent) => {
    event.preventDefault();
    if (loading) return;
    try {
      await verifySignupOtp(email, otp);
      const token = await login(email, password);
      proceedWithAuth(token);
    } catch {
      // error surfaced from hook
    }
  };

  const proceedWithAuth = (token: string) => {
    setupHost({
      type: "enterprise",
      apiKey: token,
      endpointAddress: "http://127.0.0.1:8002",
    });
  };

  const renderSignInForm = () => (
    <form onSubmit={handleSignIn} style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
      <Box>
        <Text as="label" size="2" weight="bold" style={{ display: "block", marginBottom: "8px", color: "var(--gray-11)" }}>
          Email Address
        </Text>
        <TextField.Root
          placeholder="Enter your email"
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          required
          disabled={loading}
          style={{ backgroundColor: "transparent", padding: "4px 0", width: "100%" }}
          size="3"
        >
          <TextField.Slot>
            <Mail size={16} color="var(--gray-9)" />
          </TextField.Slot>
        </TextField.Root>
      </Box>

      <Box>
        <Text as="label" size="2" weight="bold" style={{ display: "block", marginBottom: "8px", color: "var(--gray-11)" }}>
          Password
        </Text>
        <TextField.Root
          placeholder="Enter your password"
          type={showPassword ? "text" : "password"}
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          required
          disabled={loading}
          style={{ backgroundColor: "transparent", padding: "4px 0", width: "100%" }}
          size="3"
        >
          <TextField.Slot>
            <Lock size={16} color="var(--gold-9)" />
          </TextField.Slot>
          <TextField.Slot side="right">
            <Button
              type="button"
              variant="ghost"
              style={{ padding: 0, margin: 0, height: "auto" }}
              onClick={() => setShowPassword(!showPassword)}
            >
              {showPassword ? <EyeOff size={16} color="var(--gray-9)" /> : <Eye size={16} color="var(--gray-9)" />}
            </Button>
          </TextField.Slot>
        </TextField.Root>
      </Box>

      {error && (
        <Text size="2" color="red" align="center">{error}</Text>
      )}

      <Button
        type="submit"
        size="3"
        loading={loading}
        style={{ width: "100%", marginTop: "8px", backgroundColor: "#3EC6FF", color: "white", fontWeight: "bold", borderRadius: "8px", cursor: "pointer" }}
      >
        Sign In →
      </Button>
    </form>
  );

  const renderSignUpCredentialsForm = () => (
    <form onSubmit={handleSendOtp} style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
      <Box>
        <Text as="label" size="2" weight="bold" style={{ display: "block", marginBottom: "8px", color: "var(--gray-11)" }}>
          Email Address
        </Text>
        <TextField.Root
          placeholder="Enter your email"
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          required
          disabled={loading}
          style={{ backgroundColor: "transparent", padding: "4px 0", width: "100%" }}
          size="3"
        >
          <TextField.Slot>
            <Mail size={16} color="var(--gray-9)" />
          </TextField.Slot>
        </TextField.Root>
      </Box>

      <Box>
        <Text as="label" size="2" weight="bold" style={{ display: "block", marginBottom: "8px", color: "var(--gray-11)" }}>
          Password
        </Text>
        <TextField.Root
          placeholder="At least 8 characters"
          type={showPassword ? "text" : "password"}
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          required
          minLength={8}
          disabled={loading}
          style={{ backgroundColor: "transparent", padding: "4px 0", width: "100%" }}
          size="3"
        >
          <TextField.Slot>
            <Lock size={16} color="var(--gold-9)" />
          </TextField.Slot>
          <TextField.Slot side="right">
            <Button
              type="button"
              variant="ghost"
              style={{ padding: 0, margin: 0, height: "auto" }}
              onClick={() => setShowPassword(!showPassword)}
            >
              {showPassword ? <EyeOff size={16} color="var(--gray-9)" /> : <Eye size={16} color="var(--gray-9)" />}
            </Button>
          </TextField.Slot>
        </TextField.Root>
      </Box>

      <Box>
        <Text as="label" size="2" weight="bold" style={{ display: "block", marginBottom: "8px", color: "var(--gray-11)" }}>
          Confirm Password
        </Text>
        <TextField.Root
          placeholder="Re-enter your password"
          type={showPassword ? "text" : "password"}
          value={confirmPassword}
          onChange={(e) => {
            setConfirmPassword(e.target.value);
            if (credentialsError) setCredentialsError(null);
          }}
          required
          minLength={8}
          disabled={loading}
          style={{ backgroundColor: "transparent", padding: "4px 0", width: "100%" }}
          size="3"
        >
          <TextField.Slot>
            <Lock size={16} color="var(--gold-9)" />
          </TextField.Slot>
        </TextField.Root>
      </Box>

      {(credentialsError || error) && (
        <Text size="2" color="red" align="center">{credentialsError ?? error}</Text>
      )}

      <Button
        type="submit"
        size="3"
        loading={loading}
        style={{ width: "100%", marginTop: "8px", backgroundColor: "#3EC6FF", color: "white", fontWeight: "bold", borderRadius: "8px", cursor: "pointer" }}
      >
        Send Verification Code →
      </Button>
    </form>
  );

  const renderSignUpOtpForm = () => (
    <form onSubmit={handleVerifyOtp} style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
      <Flex direction="column" align="center" gap="1">
        <Text size="2" color="gray" align="center">
          A 6-digit verification code was sent to
        </Text>
        <Text size="2" weight="bold" style={{ color: "#3EC6FF" }}>
          {email}
        </Text>
      </Flex>

      <Box>
        <Text as="label" size="2" weight="bold" style={{ display: "block", marginBottom: "8px", color: "var(--gray-11)" }}>
          Verification Code
        </Text>
        <TextField.Root
          placeholder="Enter 6-digit code"
          type="text"
          inputMode="numeric"
          maxLength={6}
          value={otp}
          onChange={(e) => setOtp(e.target.value.replace(/\D/g, ""))}
          required
          disabled={loading}
          style={{ backgroundColor: "transparent", padding: "4px 0", width: "100%", letterSpacing: "0.2em", textAlign: "center" }}
          size="3"
        >
          <TextField.Slot>
            <KeyRound size={16} color="var(--gray-9)" />
          </TextField.Slot>
        </TextField.Root>
      </Box>

      {error && (
        <Text size="2" color="red" align="center">{error}</Text>
      )}

      <Button
        type="submit"
        size="3"
        loading={loading}
        style={{ width: "100%", marginTop: "8px", backgroundColor: "#3EC6FF", color: "white", fontWeight: "bold", borderRadius: "8px", cursor: "pointer" }}
      >
        Verify & Create Account →
      </Button>

      <Flex justify="center">
        <Link
          href="#"
          onClick={(e) => { e.preventDefault(); setSignUpStep("credentials"); setCredentialsError(null); }}
          style={{ color: "var(--gray-9)", fontSize: "13px" }}
        >
          ← Back
        </Link>
      </Flex>
    </form>
  );

  return (
    <Container
      size="2"
      style={{
        display: "flex",
        justifyContent: "center",
        alignItems: "center",
        height: "100dvh",
        padding: 0,
        overflow: "hidden",
        position: "relative",
      }}
    >
      <Card
        size="4"
        style={{
          width: "100%",
          maxWidth: "420px",
          margin: "0 auto",
          background: "var(--color-surface)",
          border: "1px solid var(--gray-5)",
          borderRadius: "16px",
          padding: "32px 24px",
          boxShadow: "0 8px 30px rgba(0, 0, 0, 0.5)",
        }}
      >
        <Flex direction="column" align="center" gap="4">
          <Flex align="center">
            <img
              src="/new_logo.png"
              alt="FirmGen"
              width={50}
              height={60}
              title="FirmGen"
              style={{ objectFit: "contain" }}
            />
            <Heading size="6" weight="bold">
              CraftifAI
            </Heading>
          </Flex>

          <Flex direction="column" align="center" gap="1">
            <Text size="2" color="gray">
              {isSignUp
                ? signUpStep === "otp"
                  ? "Check your inbox"
                  : "Create your FirmGen account"
                : "Sign in to CraftifAI Orbit - FirmGen"}
            </Text>
          </Flex>

          <Box style={{ width: "100%", marginTop: "16px" }}>
            {!isSignUp && renderSignInForm()}
            {isSignUp && signUpStep === "credentials" && renderSignUpCredentialsForm()}
            {isSignUp && signUpStep === "otp" && renderSignUpOtpForm()}

            {signUpStep !== "otp" && (
              <>
                <Flex align="center" gap="4" style={{ width: "100%", marginTop: "24px" }}>
                  <Box style={{ flex: 1, height: "1px", backgroundColor: "var(--gray-5)" }} />
                  <Text size="1" color="gray">OR</Text>
                  <Box style={{ flex: 1, height: "1px", backgroundColor: "var(--gray-5)" }} />
                </Flex>

                <Flex justify="center" align="center" mt="4" gap="2">
                  <Text size="2" color="gray">
                    {isSignUp ? "Already have an account?" : "Don't have an account?"}
                  </Text>
                  <Link
                    href="#"
                    onClick={handleToggleMode}
                    style={{ color: "#3EC6FF", fontWeight: "bold", textDecoration: "none" }}
                  >
                    {isSignUp ? "Sign in" : "Sign up"}
                  </Link>
                </Flex>
              </>
            )}
          </Box>
        </Flex>
      </Card>
    </Container>
  );
};
