"""Role-Based Access Control for CRA.

Provides roles, permissions, and authorization checks.
"""

from enum import Enum
from typing import Any

from pydantic import BaseModel, Field


class Permission(str, Enum):
    """Built-in permissions."""

    # Atlas permissions
    ATLAS_READ = "atlas:read"
    ATLAS_WRITE = "atlas:write"
    ATLAS_DELETE = "atlas:delete"
    ATLAS_PUBLISH = "atlas:publish"

    # Session permissions
    SESSION_CREATE = "session:create"
    SESSION_READ = "session:read"
    SESSION_DELETE = "session:delete"

    # CARP permissions
    CARP_RESOLVE = "carp:resolve"
    CARP_EXECUTE = "carp:execute"

    # TRACE permissions
    TRACE_READ = "trace:read"
    TRACE_EXPORT = "trace:export"
    TRACE_DELETE = "trace:delete"

    # Admin permissions
    ADMIN_USERS = "admin:users"
    ADMIN_ROLES = "admin:roles"
    ADMIN_POLICIES = "admin:policies"
    ADMIN_AUDIT = "admin:audit"

    # API key permissions
    APIKEY_CREATE = "apikey:create"
    APIKEY_READ = "apikey:read"
    APIKEY_REVOKE = "apikey:revoke"


class Role(BaseModel):
    """Role definition."""

    id: str
    name: str
    description: str = ""
    permissions: list[str] = Field(default_factory=list)
    inherits: list[str] = Field(default_factory=list)  # Inherit from other roles
    org_id: str | None = None  # None = global role


# Built-in roles
BUILTIN_ROLES: dict[str, Role] = {
    "admin": Role(
        id="admin",
        name="Administrator",
        description="Full system access",
        permissions=[p.value for p in Permission],
    ),
    "developer": Role(
        id="developer",
        name="Developer",
        description="Development and testing access",
        permissions=[
            Permission.ATLAS_READ.value,
            Permission.ATLAS_WRITE.value,
            Permission.SESSION_CREATE.value,
            Permission.SESSION_READ.value,
            Permission.CARP_RESOLVE.value,
            Permission.CARP_EXECUTE.value,
            Permission.TRACE_READ.value,
        ],
    ),
    "agent": Role(
        id="agent",
        name="Agent",
        description="AI agent access",
        permissions=[
            Permission.ATLAS_READ.value,
            Permission.SESSION_CREATE.value,
            Permission.CARP_RESOLVE.value,
            Permission.CARP_EXECUTE.value,
            Permission.TRACE_READ.value,
        ],
    ),
    "viewer": Role(
        id="viewer",
        name="Viewer",
        description="Read-only access",
        permissions=[
            Permission.ATLAS_READ.value,
            Permission.SESSION_READ.value,
            Permission.TRACE_READ.value,
        ],
    ),
    "auditor": Role(
        id="auditor",
        name="Auditor",
        description="Audit and compliance access",
        permissions=[
            Permission.ATLAS_READ.value,
            Permission.SESSION_READ.value,
            Permission.TRACE_READ.value,
            Permission.TRACE_EXPORT.value,
            Permission.ADMIN_AUDIT.value,
        ],
    ),
}


class RBACEngine:
    """Role-Based Access Control engine."""

    def __init__(self):
        self._roles: dict[str, Role] = dict(BUILTIN_ROLES)
        self._user_roles: dict[str, set[str]] = {}  # principal_id -> role_ids
        self._org_roles: dict[str, dict[str, Role]] = {}  # org_id -> {role_id -> Role}

    def add_role(self, role: Role) -> None:
        """Add a custom role.

        Args:
            role: The role to add
        """
        if role.org_id:
            if role.org_id not in self._org_roles:
                self._org_roles[role.org_id] = {}
            self._org_roles[role.org_id][role.id] = role
        else:
            self._roles[role.id] = role

    def get_role(self, role_id: str, org_id: str | None = None) -> Role | None:
        """Get a role by ID.

        Args:
            role_id: The role ID
            org_id: Optional organization ID for org-specific roles

        Returns:
            Role if found, None otherwise
        """
        # Check org-specific roles first
        if org_id and org_id in self._org_roles:
            if role_id in self._org_roles[org_id]:
                return self._org_roles[org_id][role_id]

        # Fall back to global roles
        return self._roles.get(role_id)

    def assign_role(self, principal_id: str, role_id: str) -> None:
        """Assign a role to a principal.

        Args:
            principal_id: The principal to assign to
            role_id: The role to assign
        """
        if principal_id not in self._user_roles:
            self._user_roles[principal_id] = set()
        self._user_roles[principal_id].add(role_id)

    def revoke_role(self, principal_id: str, role_id: str) -> None:
        """Revoke a role from a principal.

        Args:
            principal_id: The principal to revoke from
            role_id: The role to revoke
        """
        if principal_id in self._user_roles:
            self._user_roles[principal_id].discard(role_id)

    def get_principal_roles(self, principal_id: str) -> list[str]:
        """Get roles for a principal.

        Args:
            principal_id: The principal ID

        Returns:
            List of role IDs
        """
        return list(self._user_roles.get(principal_id, set()))

    def get_permissions(
        self,
        role_ids: list[str],
        org_id: str | None = None,
    ) -> set[str]:
        """Get all permissions for a set of roles.

        Args:
            role_ids: List of role IDs
            org_id: Optional organization ID

        Returns:
            Set of permission strings
        """
        permissions: set[str] = set()
        visited: set[str] = set()

        def collect_permissions(role_id: str) -> None:
            if role_id in visited:
                return
            visited.add(role_id)

            role = self.get_role(role_id, org_id)
            if not role:
                return

            permissions.update(role.permissions)

            # Process inherited roles
            for inherited_role_id in role.inherits:
                collect_permissions(inherited_role_id)

        for role_id in role_ids:
            collect_permissions(role_id)

        return permissions

    def has_permission(
        self,
        principal_id: str,
        permission: str | Permission,
        org_id: str | None = None,
        direct_roles: list[str] | None = None,
    ) -> bool:
        """Check if a principal has a permission.

        Args:
            principal_id: The principal to check
            permission: The permission to check
            org_id: Optional organization ID
            direct_roles: Optional direct role list (bypasses lookup)

        Returns:
            True if principal has the permission
        """
        if isinstance(permission, Permission):
            permission = permission.value

        # Get roles from storage or use provided
        if direct_roles is not None:
            role_ids = direct_roles
        else:
            role_ids = self.get_principal_roles(principal_id)

        # Get all permissions for these roles
        permissions = self.get_permissions(role_ids, org_id)

        return permission in permissions

    def check_permission(
        self,
        principal_id: str,
        permission: str | Permission,
        org_id: str | None = None,
        direct_roles: list[str] | None = None,
    ) -> None:
        """Check permission and raise if denied.

        Args:
            principal_id: The principal to check
            permission: The permission to check
            org_id: Optional organization ID
            direct_roles: Optional direct role list

        Raises:
            PermissionError: If permission is denied
        """
        if not self.has_permission(principal_id, permission, org_id, direct_roles):
            perm_str = permission.value if isinstance(permission, Permission) else permission
            raise PermissionError(
                f"Principal '{principal_id}' lacks permission '{perm_str}'"
            )

    def has_any_permission(
        self,
        principal_id: str,
        permissions: list[str | Permission],
        org_id: str | None = None,
        direct_roles: list[str] | None = None,
    ) -> bool:
        """Check if principal has any of the permissions.

        Args:
            principal_id: The principal to check
            permissions: List of permissions to check
            org_id: Optional organization ID
            direct_roles: Optional direct role list

        Returns:
            True if principal has any of the permissions
        """
        return any(
            self.has_permission(principal_id, p, org_id, direct_roles)
            for p in permissions
        )

    def has_all_permissions(
        self,
        principal_id: str,
        permissions: list[str | Permission],
        org_id: str | None = None,
        direct_roles: list[str] | None = None,
    ) -> bool:
        """Check if principal has all of the permissions.

        Args:
            principal_id: The principal to check
            permissions: List of permissions to check
            org_id: Optional organization ID
            direct_roles: Optional direct role list

        Returns:
            True if principal has all of the permissions
        """
        return all(
            self.has_permission(principal_id, p, org_id, direct_roles)
            for p in permissions
        )


# Singleton instance
_rbac_engine: RBACEngine | None = None


def get_rbac_engine() -> RBACEngine:
    """Get the RBAC engine singleton."""
    global _rbac_engine
    if _rbac_engine is None:
        _rbac_engine = RBACEngine()
    return _rbac_engine
