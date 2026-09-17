package net.snowflake.client.arch;

import static com.tngtech.archunit.lang.syntax.ArchRuleDefinition.classes;
import static com.tngtech.archunit.lang.syntax.ArchRuleDefinition.methods;
import static com.tngtech.archunit.lang.syntax.ArchRuleDefinition.noClasses;

import com.tngtech.archunit.core.domain.JavaClass;
import com.tngtech.archunit.core.domain.JavaClasses;
import com.tngtech.archunit.core.domain.JavaCodeUnit;
import com.tngtech.archunit.core.domain.JavaConstructorCall;
import com.tngtech.archunit.core.domain.JavaMethod;
import com.tngtech.archunit.core.importer.ClassFileImporter;
import com.tngtech.archunit.core.importer.ImportOption;
import com.tngtech.archunit.lang.ArchCondition;
import com.tngtech.archunit.lang.ConditionEvents;
import com.tngtech.archunit.lang.SimpleConditionEvent;
import java.sql.SQLException;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.api.decorator.AbstractDecorator;
import net.snowflake.client.internal.api.implementation.exception.DriverRuntimeException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.exception.SqlExceptionMapper;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.condition.DisabledForJreRange;

/**
 * Guards the runtime exception-carrier model. {@code DriverRuntimeException.toSQLException()} is
 * already compiler-enforced (a new carrier without a mapping will not compile); these rules cover
 * the structural invariants the compiler cannot. The decorator boundary invariant is behavioural,
 * not structural, and is covered by a separate reflection test.
 *
 * <p>Allowlists pin constructor sites to concrete owners: {@code SnowflakeSQLException}
 * constructors, {@code SqlExceptionMapper.translate}, {@code DriverRuntimeException.toSQLException}
 * (including overrides on subtypes), {@code SFSQLException.remoteFileNotFound}, {@code
 * LogicalConnection.abort}, and {@code SnowflakeDriver.connect}. Constructor chaining on {@code
 * SnowflakeSQLException} itself is allowed; a static factory in {@code api.exception} or a new
 * method on {@code SnowflakeDriver} / {@code SqlExceptionMapper} is still in scope.
 *
 * <p>The impl-tier {@code throws SQLException} rule is a presence check on {@code
 * SqlExceptionMapper.call} / {@code run}, not a path-sensitive wrap: any impl method that mentions
 * those mapper methods may declare {@code throws SQLException}, including a branch that throws a
 * plain {@code SQLException}. Extracting the mapper call into a helper fails this rule even when
 * the public method still goes through the mapper. {@code
 * SnowflakeResultSetSerializableImpl#getResultSet} is the production method that matches.
 *
 * <p>TODO(SNOW-3735420): {@code Driver#connect} still constructs {@code SnowflakeSQLException} for
 * an invalid URL. That allow stays until the path uses a carrier.
 *
 * <p>Disabled on JRE 22+: ArchUnit 1.3.0 (pinned for a Java 8 runtime — see jdbc/build.gradle)
 * cannot parse class files newer than Java 21, so {@code ClassFileImporter} imports zero classes
 * and every {@code should()} rule trips {@code failOnEmptyShould}. These arch invariants are
 * JVM-independent, so exercising them on the Java 8/11/17/21 lanes fully covers them; the Java 25
 * lane skips this class rather than failing on a toolchain limitation.
 */
@DisabledForJreRange(minVersion = 22)
class ExceptionModelArchTest {

  private static final JavaClasses PRODUCTION_CLASSES =
      new ClassFileImporter()
          .withImportOption(ImportOption.Predefined.DO_NOT_INCLUDE_TESTS)
          .importPackages("net.snowflake.client");

  private static final String[] IMPL_TIER_PACKAGES = {
    "net.snowflake.client.internal.api.implementation..",
    "net.snowflake.client.internal.core..",
    "net.snowflake.client.internal.util..",
  };
  private static final String EXCEPTION_PKG =
      "net.snowflake.client.internal.api.implementation.exception..";
  // Pooling wraps *already-decorated* physical connections, so it legitimately handles the checked
  // SQLException and re-surfaces it via SFSQLException.surfacing — see LogicalConnection.
  private static final String POOLING_PKG =
      "net.snowflake.client.internal.api.implementation.pooling..";
  private static final String PUBLIC_API_PKG = "net.snowflake.client.api..";
  private static final Class<?> LOGICAL_CONNECTION =
      classNamed("net.snowflake.client.internal.api.implementation.pooling.LogicalConnection");

  @Test
  void shouldNotDeclareThrowsSqlExceptionInImplTier() {
    ArchCondition<JavaMethod> leakCheckedSqlException =
        new ArchCondition<JavaMethod>("declare throws SQLException without SqlExceptionMapper") {
          @Override
          public void check(JavaMethod method, ConditionEvents events) {
            boolean leaks =
                method.getThrowsClause().getTypes().stream()
                    .anyMatch(type -> type.isAssignableTo(SQLException.class));
            if (leaks && !callsSqlExceptionMapper(method)) {
              events.add(
                  SimpleConditionEvent.violated(
                      method,
                      method.getFullName()
                          + " declares throws SQLException without calling SqlExceptionMapper"));
            }
          }
        };

    methods()
        .that()
        .areDeclaredInClassesThat()
        .resideInAnyPackage(IMPL_TIER_PACKAGES)
        .and()
        .areDeclaredInClassesThat()
        .areNotAssignableTo(AbstractDecorator.class)
        .and()
        .areDeclaredInClassesThat()
        .resideOutsideOfPackage(EXCEPTION_PKG)
        .and()
        .areDeclaredInClassesThat()
        .resideOutsideOfPackage(POOLING_PKG)
        .should(leakCheckedSqlException)
        .as("impl-tier methods should not declare throws SQLException")
        .because(
            "impl code throws DriverRuntimeException carriers; a method that must expose checked"
                + " SQLException wraps through SqlExceptionMapper.call / run rather than naming a"
                + " special class")
        .check(PRODUCTION_CLASSES);
  }

  @Test
  void shouldConstructSnowflakeSqlExceptionOnlyAtTheBoundary() {
    ArchCondition<JavaClass> constructOutsideBoundary =
        new ArchCondition<JavaClass>(
            "construct SnowflakeSQLException outside the mapper boundary") {
          @Override
          public void check(JavaClass javaClass, ConditionEvents events) {
            for (JavaConstructorCall call : javaClass.getConstructorCallsFromSelf()) {
              if (!call.getTarget().getOwner().isAssignableTo(SnowflakeSQLException.class)) {
                continue;
              }
              JavaCodeUnit origin = call.getOrigin();
              if (allowedToConstructSnowflakeSqlException(origin)) {
                continue;
              }
              events.add(
                  SimpleConditionEvent.violated(
                      call,
                      origin.getFullName()
                          + " constructs SnowflakeSQLException; only SnowflakeSQLException"
                          + " constructors, SqlExceptionMapper.translate,"
                          + " DriverRuntimeException.toSQLException,"
                          + " SFSQLException.remoteFileNotFound, LogicalConnection.abort, or"
                          + " SnowflakeDriver.connect may"));
            }
          }
        };

    classes()
        .should(constructOutsideBoundary)
        .as("only the exception boundary should construct SnowflakeSQLException")
        .because(
            "byte-exact construction funnels through SqlExceptionMapper.translate / carrier"
                + " toSQLException; a factory in api.exception or a new Driver / mapper method is"
                + " not exempt")
        .check(PRODUCTION_CLASSES);
  }

  @Test
  void shouldMakeEveryExceptionCarrierUnchecked() {
    classes()
        .that()
        .resideInAPackage(EXCEPTION_PKG)
        .and()
        .areAssignableTo(Throwable.class)
        .should()
        .beAssignableTo(DriverRuntimeException.class)
        .as("throwables in the exception package should extend DriverRuntimeException")
        .because(
            "a checked carrier would force throws SQLException back into the impl tier it was"
                + " removed from")
        .check(PRODUCTION_CLASSES);
  }

  @Test
  void shouldNotLeakCarrierTypesThroughThePublicApi() {
    noClasses()
        .that()
        .resideInAPackage(PUBLIC_API_PKG)
        .should()
        .dependOnClassesThat()
        .areAssignableTo(DriverRuntimeException.class)
        .as("public api classes should not depend on internal carrier types")
        .because(
            "carriers are translated to SnowflakeSQLException before crossing the api boundary")
        .check(PRODUCTION_CLASSES);
  }

  private static boolean callsSqlExceptionMapper(JavaMethod method) {
    return method.getMethodCallsFromSelf().stream()
        .anyMatch(
            call ->
                call.getTarget().getOwner().isAssignableTo(SqlExceptionMapper.class)
                    && ("call".equals(call.getName()) || "run".equals(call.getName())));
  }

  private static boolean allowedToConstructSnowflakeSqlException(JavaCodeUnit origin) {
    JavaClass owner = origin.getOwner();
    String name = origin.getName();
    if (owner.isEquivalentTo(SnowflakeSQLException.class) && "<init>".equals(name)) {
      return true;
    }
    if (owner.isEquivalentTo(SqlExceptionMapper.class) && "translate".equals(name)) {
      return true;
    }
    if (owner.isAssignableTo(DriverRuntimeException.class) && "toSQLException".equals(name)) {
      return true;
    }
    if (owner.isEquivalentTo(SFSQLException.class) && "remoteFileNotFound".equals(name)) {
      return true;
    }
    if (owner.isEquivalentTo(LOGICAL_CONNECTION) && "abort".equals(name)) {
      return true;
    }
    return owner.isEquivalentTo(SnowflakeDriver.class) && "connect".equals(name);
  }

  private static Class<?> classNamed(String name) {
    try {
      return Class.forName(name);
    } catch (ClassNotFoundException e) {
      throw new ExceptionInInitializerError(e);
    }
  }
}
