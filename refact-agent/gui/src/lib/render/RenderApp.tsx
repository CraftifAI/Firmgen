import { StrictMode, useEffect } from "react";
import { type Config, updateConfig } from "../../features/Config/configSlice";
import { App } from "../../features/App";
import ReactDOM from "react-dom/client";
import { store } from "../../app/store";
import "./web.css";

export function renderApp(element: HTMLElement, config: Config) {
  // Store config in a ref to avoid closure issues
  const configRef = { current: config };

  const AppWrapped: React.FC = () => {
    // Update Redux store with the initial config after component mounts
    useEffect(() => {
      store.dispatch(updateConfig(configRef.current));
    }, []);

    return (
      <StrictMode>
        <App />
      </StrictMode>
    );
  };
  ReactDOM.createRoot(element).render(<AppWrapped />);
}
