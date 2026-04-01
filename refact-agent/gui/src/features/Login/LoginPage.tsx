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
import { User, Mail, Lock, Eye, EyeOff } from "lucide-react";
import { useCraftifAuth } from "../../hooks";
import { useEventsBusForIDE } from "../../hooks";

export const LoginPage: React.FC = () => {
  const { login, register, loading, error } = useCraftifAuth();
  const { setupHost } = useEventsBusForIDE();

  const [isSignUp, setIsSignUp] = useState(false);
  const [showPassword, setShowPassword] = useState(false);

  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  const handleToggleMode = () => {
    setIsSignUp(!isSignUp);
    setUsername("");
    setEmail("");
    setPassword("");
  };

  const handleAuth = async (event: React.FormEvent) => {
    event.preventDefault();
    if (loading) return;

    try {
      if (isSignUp) {
        await register(email, password);
        const token = await login(email, password, username);
        proceedWithAuth(token);
      } else {
        const token = await login(email, password, username);
        proceedWithAuth(token);
      }
    } catch {
      // Error is handled in the hook and displayed below
    }
  };

  const proceedWithAuth = (token: string) => {
    setupHost({
      type: "enterprise",
      apiKey: token,
      endpointAddress: "http://127.0.0.1:8002",
    });
  };

  return (
    <Container size="2" style={{ display: "flex", justifyContent: "center", alignItems: "center", minHeight: "100vh" }}>
      <Card
        size="4"
        style={{
          width: "100%",
          maxWidth: "420px",
          background: "var(--color-surface)",
          border: "1px solid var(--gray-5)",
          borderRadius: "16px",
          padding: "32px 24px",
          boxShadow: "0 8px 30px rgba(0, 0, 0, 0.5)",
        }}
      >
        <Flex direction="column" align="center" gap="4">
          <img
            src="/new_logo.png"
            alt="FirmGen"
            width={50}
            height={60}
            style={{ objectFit: "contain" }}
          />

          <Flex direction="column" align="center" gap="1">
            <Heading size="6" weight="bold">
              Welcome
            </Heading>
            <Text size="2" color="gray">
              Sign in to CraftifAI Orbit - FirmGen
            </Text>
          </Flex>

          <Box style={{ width: "100%", marginTop: "16px" }}>
            <form onSubmit={handleAuth} style={{ display: "flex", flexDirection: "column", gap: "20px" }}>

              <Box>
                <Text as="label" size="2" weight="bold" style={{ display: "block", marginBottom: "8px", color: "var(--gray-11)" }}>
                  Username
                </Text>
                <TextField.Root
                  placeholder="Enter your username"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  disabled={loading}
                  style={{ backgroundColor: "transparent", padding: "4px 0" }}
                  size="3"
                >
                  <TextField.Slot>
                    <User size={16} color="var(--blue-9)" />
                  </TextField.Slot>
                </TextField.Root>
              </Box>

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
                  style={{ backgroundColor: "transparent", padding: "4px 0" }}
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
                  style={{ backgroundColor: "transparent", padding: "4px 0" }}
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
                <Text size="2" color="red" align="center">
                  {error}
                </Text>
              )}

              <Button
                type="submit"
                size="3"
                loading={loading}
                style={{
                  width: "100%",
                  marginTop: "8px",
                  backgroundColor: "var(--teal-9)",
                  color: "white",
                  fontWeight: "bold",
                  borderRadius: "8px",
                  cursor: "pointer",
                }}
              >
                {isSignUp ? "Sign Up \u2192" : "Sign In \u2192"}
              </Button>
            </form>

            <Flex align="center" gap="4" style={{ width: "100%", marginTop: "24px" }}>
              <Box style={{ flex: 1, height: "1px", backgroundColor: "var(--gray-5)" }} />
              <Text size="1" color="gray">
                OR
              </Text>
              <Box style={{ flex: 1, height: "1px", backgroundColor: "var(--gray-5)" }} />
            </Flex>

            <Flex justify="center" align="center" mt="4" gap="2">
              <Text size="2" color="gray">
                {isSignUp ? "Already have an account?" : "Don't have an account?"}
              </Text>
              <Link
                href="#"
                onClick={(e) => {
                  e.preventDefault();
                  handleToggleMode();
                }}
                style={{
                  color: "var(--teal-9)",
                  fontWeight: "bold",
                  textDecoration: "none"
                }}
              >
                {isSignUp ? "Sign in" : "Sign up"}
              </Link>
            </Flex>
          </Box>
        </Flex>
      </Card>

      <Box style={{ position: "absolute", bottom: "16px", width: "100%", textAlign: "center" }}>
        <Text size="1" color="gray">
          FirmGen v20260313_1900
        </Text>
      </Box>
    </Container>
  );
};
