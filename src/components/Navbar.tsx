import React from "react";

interface NavbarProps {
  onConnectWallet: () => void;
}

const Navbar: React.FC<NavbarProps> = ({ onConnectWallet }) => {
  const navbarStyle: React.CSSProperties = {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    padding: "1rem",
    backgroundColor: "#f8f9fa",
    boxShadow: "0 2px 4px rgba(0,0,0,0.1)",
  };

  const titleStyle: React.CSSProperties = {
    fontSize: "1.5rem",
    fontWeight: "bold",
    color: "#333",
  };

  const navLinksStyle: React.CSSProperties = {
    display: "flex",
    gap: "1rem",
  };

  const linkStyle: React.CSSProperties = {
    color: "#333",
    textDecoration: "none",
    padding: "0.5rem",
    borderRadius: "4px",
    transition: "background-color 0.3s",
  };

  const buttonStyle: React.CSSProperties = {
    padding: "0.5rem 1rem",
    backgroundColor: "#007bff",
    color: "white",
    border: "none",
    borderRadius: "4px",
    cursor: "pointer",
  };

  return (
    <nav style={navbarStyle}>
      <div style={titleStyle}>RunicSwap</div>
      <div style={navLinksStyle}>
        <a href="#swap" style={linkStyle}>
          Swap
        </a>
        <a href="#pools" style={linkStyle}>
          Pools
        </a>
        <a href="#position" style={linkStyle}>
          Position
        </a>
      </div>
      <button style={buttonStyle} onClick={onConnectWallet}>
        Connect Wallet
      </button>
    </nav>
  );
};

export default Navbar;
