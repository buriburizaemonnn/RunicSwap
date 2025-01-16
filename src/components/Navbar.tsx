import React, { useState } from "react";
import { Link, useLocation } from "react-router-dom";
import { useIdentity } from "../contexts/IdentityContext";
import ConnectDialog from "./ConnectDialog";

import "./../styles/Navbar.css";

const Navbar: React.FC = () => {
  const location = useLocation();
  const { identity, logout } = useIdentity();
  const [isConnectDialogOpen, setIsConnectDialogOpen] = useState(false);
  const [isDropdownOpen, setIsDropdownOpen] = useState(false);

  const isActive = (path: string) => {
    return location.pathname === path ? "navbar-link active" : "navbar-link";
  };

  const toggleDropdown = () => {
    setIsDropdownOpen(!isDropdownOpen);
  };

  const handleDisconnect = async () => {
    await logout();
    setIsDropdownOpen(false);
  };

  const getDisplayAddress = () => {
    return "Connected";
  };

  return (
    <nav className="navbar">
      <div className="navbar-container">
        <div className="navbar-content">
          <div className="navbar-left">
            <div className="navbar-brand">
              <span className="navbar-logo">RunicSwap</span>
            </div>
            <div className="navbar-links">
              <Link to="/swap" className={isActive("/swap")}>
                Swap
              </Link>
              <Link to="/pools" className={isActive("/pools")}>
                Pools
              </Link>
              <Link to="/positions" className={isActive("/positions")}>
                Positions
              </Link>
            </div>
          </div>
          <div className="navbar-actions">
            {identity ? (
              <div className="navbar-dropdown">
                <button
                  className="navbar-dropdown-toggle"
                  onClick={toggleDropdown}
                >
                  {getDisplayAddress()}
                </button>
                {isDropdownOpen && (
                  <div className="navbar-dropdown-menu">
                    <Link to="/vault" className="navbar-dropdown-item">
                      Vault
                    </Link>
                    <button
                      className="navbar-dropdown-item"
                      onClick={handleDisconnect}
                    >
                      Disconnect
                    </button>
                  </div>
                )}
              </div>
            ) : (
              <button
                className="navbar-connect-button"
                onClick={() => setIsConnectDialogOpen(true)}
              >
                Connect
              </button>
            )}
          </div>
        </div>
      </div>
      <ConnectDialog
        isOpen={isConnectDialogOpen}
        setIsOpen={setIsConnectDialogOpen}
      />
    </nav>
  );
};

export default Navbar;
