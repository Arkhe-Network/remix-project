import numpy as np

class Quaternion:
    def __init__(self, w, x, y, z):
        self.w, self.x, self.y, self.z = w, x, y, z

    @staticmethod
    def from_axis_angle(axis, angle):
        axis = np.array(axis) / np.linalg.norm(axis)
        s = np.sin(angle / 2)
        return Quaternion(np.cos(angle / 2), axis[0]*s, axis[1]*s, axis[2]*s)

    def rotate_vector(self, v):
        q_vec = np.array([self.x, self.y, self.z])
        v_q = np.array(v)
        return v_q + 2 * np.cross(q_vec, np.cross(q_vec, v_q) + self.w * v_q)

class DigitalTwinAgent:
    def execute(self, state):
        # Retrieve spacecraft attitude
        q_att = Quaternion(*state.get('spacecraft_quat', [1, 0, 0, 0]))

        # External torque from radiation pressure (estimated from radiation_hardness score)
        space_scores = state.get('space_scores', [{}])
        if space_scores:
            rad_score = space_scores[0].get('radiation_hardness', 0.5)
        else:
            rad_score = 0.5
        drift_rate = (1 - rad_score) * 1e-5  # rad/s per thermal cycle

        # Update quaternion using Hamiltonian kinematics
        omega = np.array([drift_rate, drift_rate * 0.3, 0])  # anisotropic drift
        omega_norm = np.linalg.norm(omega)
        if omega_norm > 0:
            q_delta = Quaternion.from_axis_angle(omega / omega_norm, omega_norm)
            q_new = self._slerp(q_att, q_delta, 0.01)  # SLERP integration
        else:
            q_new = q_att

        # Project the topological invariant axis (e.g., altermagnetic vector) into lab frame
        altermag_axis = np.array([0, 0, 1])  # c-axis of EuIn2As2
        lab_axis = q_new.rotate_vector(altermag_axis)

        state['digital_twin_state'] = {
            "quaternion": [q_new.w, q_new.x, q_new.y, q_new.z],
            "lab_orientation": lab_axis.tolist(),
            "mzm_stability": self._compute_mzm_stability(lab_axis, rad_score)
        }
        return state

    def _slerp(self, qa, qb, t):
        cosom = qa.w*qb.w + qa.x*qb.x + qa.y*qb.y + qa.z*qb.z
        if cosom < 0.0:
            cosom = -cosom
            qb = Quaternion(-qb.w, -qb.x, -qb.y, -qb.z)
        if (1.0 - cosom) > 1e-6:
            omega = np.arccos(cosom)
            sinom = np.sin(omega)
            w = np.sin((1-t)*omega)/sinom * qa.w + np.sin(t*omega)/sinom * qb.w
            x = np.sin((1-t)*omega)/sinom * qa.x + np.sin(t*omega)/sinom * qb.x
            y = np.sin((1-t)*omega)/sinom * qa.y + np.sin(t*omega)/sinom * qb.y
            z = np.sin((1-t)*omega)/sinom * qa.z + np.sin(t*omega)/sinom * qb.z
            return Quaternion(w, x, y, z)
        else:
            return Quaternion(qa.w*(1-t)+qb.w*t, qa.x*(1-t)+qb.x*t, qa.y*(1-t)+qb.y*t, qa.z*(1-t)+qb.z*t)

    def _compute_mzm_stability(self, lab_axis, rad_score):
        # Dummy stability computation based on orientation and radiation hardness
        alignment = abs(np.dot(lab_axis, [0, 0, 1]))
        return float(alignment * rad_score)
